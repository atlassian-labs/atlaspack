// @flow strict-local

import type {Asset} from '@atlaspack/types';
import type {PackageNode, PackagedDominatorGraph} from './createPackages';
import {type NodeId} from '@atlaspack/graph';
import {getGraphPostOrder} from './findAssetDominators';
import type {StronglyConnectedComponentNode} from './cycleBreaker';
import type {AssetNode, SimpleAssetGraph} from './bundleGraphToRootedGraph';
import {EdgeContentGraph} from './EdgeContentGraph';
import {ALL_EDGE_TYPES} from '@atlaspack/graph/src';

function getAssetSize(asset: Asset): number {
  return asset.stats?.size ?? 0;
}

export type PackageInfo = {|totalSize: number, assets: Asset[]|};

export function getPackageInformation(
  packages: PackagedDominatorGraph,
  packageNodeId: NodeId,
  packageNode:
    | AssetNode
    | PackageNode
    | StronglyConnectedComponentNode<AssetNode>,
): PackageInfo {
  const assets: Asset[] = [];
  let totalSize = 0;
  if (packageNode.type === 'StronglyConnectedComponent') {
    for (const {asset} of packageNode.values) {
      assets.push(asset);
      totalSize += getAssetSize(asset);
    }
  } else if (packageNode.type === 'asset') {
    assets.push(packageNode.asset);
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
        assets.push(assetNode.asset);
      }
      return;
    }

    if (node.type === 'asset') {
      const assetSize = getAssetSize(node.asset);
      totalSize += assetSize;
      assets.push(node.asset);
    }
  }, packageNodeId);

  return {totalSize, assets};
}

/**
 * Build a graph that represents the dependencies between packages.
 */
export function buildPackageGraph(
  assetGraph: SimpleAssetGraph,
  packages: PackagedDominatorGraph,
  packageNodes: Array<NodeId>,
  packageInfos: Array<null | PackageInfo>,
): PackagedDominatorGraph {
  const packagesByAssetId: Map<string, NodeId> = new Map();
  packageInfos.forEach((packageInfo, idx) => {
    if (packageInfo == null) {
      return;
    }
    for (const asset of packageInfo.assets) {
      packagesByAssetId.set(asset.id, packageNodes[idx]);
    }
  });

  const packageGraph: PackagedDominatorGraph = new EdgeContentGraph();
  const rootId = packageGraph.addNodeByContentKey('root', 'root');
  packageGraph.setRootNodeId(rootId);
  const packageNodesSet = new Set(packageNodes);
  packages.nodes.forEach((node, nodeId) => {
    if (node === 'root') {
      return;
    }

    if (node == null || !packageNodesSet.has(nodeId)) {
      // $FlowFixMe
      packageGraph.addNode(null);
      return;
    }

    const contentKey = packages.getContentKeyByNodeId(nodeId);
    const packageId = packageGraph.addNodeByContentKey(contentKey, node);
    packageGraph.addEdge(rootId, packageId);
  });

  packageInfos.forEach((packageInfo, idx) => {
    if (packageInfo == null) {
      return;
    }

    const packageNodeId = packageNodes[idx];
    const packageNode = packages.getNode(packageNodeId);
    if (packageNode == null || packageNode === 'root') {
      return;
    }

    for (let asset of packageInfo.assets) {
      assetGraph.forEachNodeIdConnectedTo(
        assetGraph.getNodeIdByContentKey(asset.id),
        (nodeId) => {
          const node = assetGraph.getNode(nodeId);
          if (node == null || node === 'root') {
            return;
          }

          const packageId = packagesByAssetId.get(node.id);
          if (packageId == null) {
            return;
          }

          if (packageId === packageNodeId) {
            return;
          }

          packageGraph.addEdge(packageId, packageNodeId);
        },
        ALL_EDGE_TYPES,
      );
    }
  });

  return packageGraph;
}

export function buildPackageInfos(packages: PackagedDominatorGraph): {|
  packageNodes: Array<NodeId>,
  packageInfos: Array<null | PackageInfo>,
|} {
  const packageNodes = packages.getNodeIdsConnectedFrom(
    packages.getNodeIdByContentKey('root'),
  );
  const packageInfos: (PackageInfo | null)[] = packageNodes.map(
    (nodeId): PackageInfo | null => {
      const packageNode = packages.getNode(nodeId);
      if (packageNode == null || packageNode === 'root') {
        return null;
      }
      return getPackageInformation(packages, nodeId, packageNode);
    },
  );
  return {packageNodes, packageInfos};
}

/**
 * Merge packages whenever doing it will not increase the total size of all
 * bundles by more than a given threshold, do this iteratively starting at
 * the leaf packages.
 */
export function runMergePackages(
  assetGraph: SimpleAssetGraph,
  packages: PackagedDominatorGraph,
): PackagedDominatorGraph {
  const {packageNodes, packageInfos} = buildPackageInfos(packages);
  const packageInfosById: Map<string, PackageInfo> = new Map();
  packageNodes.forEach((nodeId, idx) => {
    const packageInfo = packageInfos[idx];
    if (packageInfo == null) {
      return;
    }
    const node = packages.getNode(nodeId);
    if (node == null || node === 'root') {
      return;
    }
    packageInfosById.set(node.id, packageInfo);
  });

  const packageGraph = buildPackageGraph(
    assetGraph,
    packages,
    packageNodes,
    packageInfos,
  );

  return mergePackageNodes(packages, packageGraph, packageInfosById).output;
}

export function mergePackageNodes(
  packages: PackagedDominatorGraph,
  packageGraph: PackagedDominatorGraph,
  packageInfosById: Map<string, PackageInfo>,
): {|
  output: PackagedDominatorGraph,
  totalSizeIncrease: number,
|} {
  const output: PackagedDominatorGraph = packages.clone();

  const postOrder = getGraphPostOrder(packageGraph);

  const removedNodes = new Set();

  let totalSizeIncrease = 0;
  for (let id of postOrder) {
    const node = packages.getNode(id);
    if (node == null || node === 'root') {
      continue;
    }

    const packageInfo: PackageInfo | null | void = packageInfosById.get(
      node.id,
    );
    if (packageInfo == null) {
      continue;
    }

    const totalSize = packageInfo.totalSize;
    const parents = [];
    packageGraph.forEachNodeIdConnectedTo(
      id,
      (parent) => {
        const contentKey = packageGraph.getContentKeyByNodeId(parent);
        if (contentKey === 'root') {
          return;
        }
        parents.push(contentKey);
      },
      ALL_EDGE_TYPES,
    );

    const sizeIncreaseFromDuplication = totalSize; // * parents.length;
    if (parents.length && sizeIncreaseFromDuplication < 1024 * 10) {
      const directChildren = packages.getNodeIdsConnectedFrom(id);
      for (let parent of parents) {
        const parentInfo = packageInfosById.get(parent);
        if (parentInfo == null) {
          continue;
        }

        parentInfo.totalSize += totalSize;
        parentInfo.assets.push(...packageInfo.assets);

        for (let child of directChildren) {
          if (!removedNodes.has(parent)) {
            const parentId = output.getNodeIdByContentKey(parent);
            output.addEdge(parentId, child);
          }
        }
      }

      output.removeNode(id, false);
      removedNodes.add(node.id);
      totalSizeIncrease += sizeIncreaseFromDuplication * parents.length;
    }
  }

  return {output, totalSizeIncrease};
}
