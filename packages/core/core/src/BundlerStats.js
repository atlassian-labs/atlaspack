// @flow strict-local

import type {Asset} from '@atlaspack/types';
import type {AtlaspackOptions} from './types';
import type {
  PackagedDominatorGraph,
  AssetDominatorTree,
  PackageNode,
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
  packageNode: Asset | PackageNode,
): PackageSummary {
  let rootAsset = undefined;
  const assets = [];
  let totalSize = 0;

  if (packageNode.type !== 'package') {
    rootAsset = packageNode.filePath?.replace(projectRoot, '');
    totalSize += getAssetSize(packageNode);
  }

  packages.traverse((nodeId) => {
    const node = packages.getNode(nodeId);
    if (node !== 'root' && node != null && node.type !== 'package') {
      const assetSize = getAssetSize(node);
      totalSize += assetSize;
      assets.push({
        filePath: node.filePath?.replace(projectRoot, ''),
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

export function runGetBundlerStats({
  dominators,
  packages,
  resolvedOptions,
}: RunGetBundlerStatsParams) {
  const chunks = dominators.getNodeIdsConnectedFrom(
    dominators.getNodeIdByContentKey('root'),
  );
  log('==> number of dominator chunks: ' + chunks.length);
  const packageNodes = packages.getNodeIdsConnectedFrom(
    packages.getNodeIdByContentKey('root'),
  );
  log('==> number of packages: ' + packageNodes.length);

  const projectRoot = resolvedOptions.projectRoot;

  log('Aggregating package stats...');
  const packageArray = [];
  for (let packageNodeId of packageNodes) {
    let packageNode = packages.getNode(packageNodeId);
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

  const bundlerStats = {
    packages: packageArray,
    stats,
  };

  const outputPath = path.join(projectRoot, './packages.json');
  log('Writing package stats to ' + outputPath);
  fs.writeFileSync(outputPath, JSON.stringify(bundlerStats, null, 2));
}
