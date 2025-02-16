// @flow strict-local

import type {Asset} from '@atlaspack/types';
import type {AtlaspackOptions} from './types';
import {
  type PackagedDominatorGraph,
  type AssetDominatorTree,
  type PackageNode,
  type AssetNode,
  type StronglyConnectedComponentNode,
  getPackageNodes,
} from '@atlaspack/bundler-experimental';
import {hashString} from '@atlaspack/rust';
import logger from '@atlaspack/logger';
import fs from 'fs';
import type {NodeId} from '@atlaspack/graph';
import path from 'path';

const log = (message) => {
  logger.info({
    message,
    origin: '@atlaspack/core',
  });
};

export interface RunGetBundlerStatsParams {
  dominators: AssetDominatorTree;
  packages: PackagedDominatorGraph;
  mergedPackages: PackagedDominatorGraph;
  resolvedOptions: AtlaspackOptions;
}

function getAssetSize(asset: Asset): number {
  return asset.stats?.size ?? 0;
}

export interface AssetSummary {
  filePath: string;
  size: number;
}

export interface PackageSummary {
  id: string;
  rootAsset?: string;
  assets: AssetSummary[];
  totalSize: number;
}

function createPackageSummary(
  projectRoot: string,
  packages: PackagedDominatorGraph,
  packageNodeId: NodeId,
  packageNode:
    | AssetNode
    | PackageNode
    | StronglyConnectedComponentNode<AssetNode>,
): PackageSummary {
  let rootAsset = undefined;
  const assets = [];
  let totalSize = 0;

  if (packageNode.type === 'asset') {
    rootAsset = packageNode.asset.filePath?.replace(projectRoot, '');
    totalSize += getAssetSize(packageNode.asset);
  }

  packages.traverse((nodeId) => {
    const node = packages.getNode(nodeId);
    if (node === 'root' || node == null) {
      return;
    }

    if (node.type === 'StronglyConnectedComponent') {
      for (const assetNode of node.values) {
        const assetSize = getAssetSize(assetNode.asset);
        totalSize += assetSize;
        assets.push({
          filePath: assetNode.asset.filePath?.replace(projectRoot, ''),
          size: assetSize,
        });
      }
      return;
    }

    if (node.type === 'asset') {
      const assetSize = getAssetSize(node.asset);
      totalSize += assetSize;
      assets.push({
        filePath: node.asset.filePath?.replace(projectRoot, ''),
        size: assetSize,
      });
    }
  }, packageNodeId);

  return {
    id:
      packageNode.type === 'package'
        ? `package:${hashString(packageNode.id)}`
        : `asset:${packageNode.id}`,
    rootAsset,
    assets,
    totalSize,
  };
}

export interface GetPackageStatsParams {
  chunks: NodeId[];
  packages: PackagedDominatorGraph;
  resolvedOptions: AtlaspackOptions;
}

export function getPackagesStats({
  chunks,
  packages,
  resolvedOptions,
}: GetPackageStatsParams): {|
  packageArray: PackageSummary[],
  stats: {|
    numPackages: number,
    numChunks: number,
    totalPackageSize: number,
    averagePackageSize: number,
    maximumPackageSize: number,
    minimumPackageSize: number,
    packageSizeDistribution: {|
      p10: number,
      p25: number,
      p50: number,
      p75: number,
      p90: number,
      p99: number,
    |},
  |},
|} {
  log('==> number of dominator chunks: ' + chunks.length);
  const packageNodes = getPackageNodes(packages);
  log('==> number of packages: ' + packageNodes.length);

  const projectRoot = resolvedOptions.projectRoot;

  log('Aggregating package stats...');
  const packageArray = [];
  for (let packageNodeId of packageNodes) {
    const packageNode = packages.getNode(packageNodeId);
    if (packageNode == null || packageNode === 'root') {
      continue;
    }

    packageArray.push(
      createPackageSummary(projectRoot, packages, packageNodeId, packageNode),
    );
  }

  const packageArraySorted = packageArray.sort(
    (a, b) => a.totalSize - b.totalSize,
  );
  const percentileSize = (percentile) => {
    const index = Math.floor((percentile / 100) * packageArraySorted.length);
    return packageArraySorted[index].totalSize;
  };

  const p10 = percentileSize(10);
  const p25 = percentileSize(25);
  const p50 = percentileSize(50);
  const p75 = percentileSize(75);
  const p90 = percentileSize(90);
  const p99 = percentileSize(99);

  const stats = {
    numPackages: packageArray.length,
    numChunks: chunks.length,
    totalPackageSize: packageArray.reduce((acc, pkg) => acc + pkg.totalSize, 0),
    averagePackageSize:
      packageArray.reduce((acc, pkg) => acc + pkg.totalSize, 0) /
      packageArray.length,
    maximumPackageSize: packageArray.reduce(
      (acc, pkg) => Math.max(acc, pkg.totalSize),
      0,
    ),
    minimumPackageSize: packageArray.reduce(
      (acc, pkg) => Math.min(acc, pkg.totalSize),
      Infinity,
    ),
    packageSizeDistribution: {
      p10,
      p25,
      p50,
      p75,
      p90,
      p99,
    },
  };

  return {
    packageArray,
    stats,
  };
}

export function runGetBundlerStats({
  dominators,
  packages,
  mergedPackages,
  resolvedOptions,
}: RunGetBundlerStatsParams) {
  const chunks = dominators.getNodeIdsConnectedFrom(
    dominators.getNodeIdByContentKey('root'),
  );

  const packageStats = getPackagesStats({packages, chunks, resolvedOptions});
  const mergedPackageStats = getPackagesStats({
    packages: mergedPackages,
    chunks,
    resolvedOptions,
  });

  const bundlerStats = {
    packageStats,
    mergedPackageStats,
  };

  const projectRoot = resolvedOptions.projectRoot;
  const outputPath = path.join(projectRoot, './packages.json');
  log('Writing package stats to ' + outputPath);

  fs.writeFileSync(outputPath, JSON.stringify(bundlerStats, null, 2));
}
