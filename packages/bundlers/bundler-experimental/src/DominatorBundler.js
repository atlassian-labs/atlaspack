// @flow strict-local

import type {
  BundleGraph,
  Bundle,
  MutableBundleGraph,
  Asset,
  Dependency,
} from '@atlaspack/types';
import {Graph, ContentGraph} from '@atlaspack/graph';
import nullthrows from 'nullthrows';

export type DominatorBundlerInput<B: Bundle> = {|
  inputGraph: BundleGraph<B>,
  outputGraph: MutableBundleGraph,
  entries: {|entryAsset: Asset, entryDependency: Dependency|}[],
|};

export type DominatorBundlerOutput = {|
  bundleGraph: BundleGraph<Bundle>,
|};

export function dominatorBundler<B: Bundle>({
  inputGraph,
  outputGraph,
  entries,
}: DominatorBundlerInput<B>): DominatorBundlerOutput {
  throw new Error('');
}

export function getImmediateDominatorTree(
  root: Asset,
  dominators: Map<Asset, Set<Asset>>,
): Map<Asset, Set<Asset>> {
  const dominanceRelationGraph = new Graph<Asset>();
  const assetNodeIds = new Map<Asset, number>();
  const nodeIdsAssets = new Map<number, Asset>();

  for (let [asset] of dominators) {
    const nodeId = dominanceRelationGraph.addNode(asset);
    assetNodeIds.set(asset, nodeId);
    nodeIdsAssets.set(nodeId, asset);
    if (asset.id === root.id) {
      dominanceRelationGraph.setRootNodeId(nodeId);
    }
  }

  for (let [asset, dominatedSet] of dominators) {
    const nodeId = nullthrows(assetNodeIds.get(asset));
    for (let dominated of dominatedSet) {
      const dominatedNodeId = nullthrows(assetNodeIds.get(dominated));
      dominanceRelationGraph.addEdge(dominatedNodeId, nodeId);
    }
  }

  const dominatedByMap = new Map<Asset, Set<Asset>>();
  for (let [asset, dominatedSet] of dominators) {
    for (let dominated of dominatedSet) {
      let dominatedBy = dominatedByMap.get(dominated);
      if (dominatedBy == null) {
        dominatedBy = new Set();
        dominatedByMap.set(dominated, dominatedBy);
      }
      dominatedBy.add(asset);
    }
  }

  const immediateDominatorTree = new Map();
  const orderedNodes = dominanceRelationGraph.topoSort();
  orderedNodes.reverse();

  for (let node of orderedNodes) {
    const asset = nullthrows(nodeIdsAssets.get(node));
    const dominates = new Set();
    const immdiateNodeId = immediateDominatorTree.set(asset, dominates);
    if (asset === root) {
      dominanceRelationGraph.setRootNodeId(immdiateNodeId);
    }
  }

  return immediateDominatorTree;
}

/**
 * For all assets, build the dominance relationship of the asset with other
 * assets.
 *
 * This implements "A simple, fast dominance algorithm" - https://www.cs.tufts.edu/comp/150FP/archive/keith-cooper/dom14.pdf
 *
 * The return value contains the **immediate** dominator tree, which is
 * different to the dominator tree.
 */
export function findAssetDominators<B: Bundle>(
  bundleGraph: BundleGraph<B>,
  roots: Asset[],
): Map<Asset, Set<Asset>> {
  const dominators = new Map<Asset, Set<Asset>>();
  bundleGraph.traverse((node) => {
    if (node.type === 'asset') {
      dominators.set(node.value, new Set());
    }
  });

  // All nodes dominates themselves initially
  for (let asset of dominators.keys()) {
    dominators.get(asset)?.add(asset);
  }

  // Build a simpler graph with a root at the top
  const graph = new ContentGraph();
  const rootNodeId = graph.addNode({type: 'root'});
  bundleGraph.traverse((node) => {
    if (node.type === 'asset') {
      graph.addNodeByContentKey(node.value.id, node.value);
    } else if (node.type === 'dependency' && node.value.isEntry) {
      const asset = bundleGraph.getAssetById(
        nullthrows(node.value.sourceAssetId),
      );
      const assetNodeId = graph.getNodeIdByContentKey(asset.id);
      graph.addEdge(rootNodeId, assetNodeId);
    }
  });
  bundleGraph.traverse((node) => {
    if (node.type === 'asset') {
      const dependencies = bundleGraph.getDependencies(node.value);

      for (let dependency of dependencies) {
        graph.addEdge(
          graph.getNodeIdByContentKey(node.value.id),
          graph.getNodeIdByContentKey(dependency.id),
        );
      }
    }
  });

  // let changed = true;
  // while (changed) {
  //   changed = false;

  //   graph.traverse((node) => {
  //     if (node.type === 'asset') {
  //       const asset = node.value;
  //       if (asset === root) {
  //         return;
  //       }
  //       const predecessors = graph
  //         .getIncomingDependencies(asset)
  //         .filter((dep) => !dep.isEntry)
  //         .map((dep) => {
  //           const asset = graph.getAssetWithDependency(dep);
  //           if (asset == null) {
  //             throw new Error('Asset not found');
  //           }
  //           return asset;
  //         });
  //       const newDominators = new Set([asset]);
  //       if (predecessors.length > 0) {
  //         const [firstAsset, ...otherAssets] = predecessors.map((asset) =>
  //           nullthrows(dominators.get(asset)),
  //         );
  //         const intersection = new Set(
  //           [...firstAsset].filter((x) => otherAssets.every((r) => r.has(x))),
  //         );
  //         for (let dominated of intersection) {
  //           newDominators.add(dominated);
  //         }
  //       }
  //       if (!isSetEqual(nullthrows(dominators.get(asset)), newDominators)) {
  //         changed = true;
  //         dominators.set(asset, newDominators);
  //       }
  //     }
  //   });
  // }

  return dominators;
}

export function isSetEqual(a: Set<Asset>, b: Set<Asset>) {
  if (a.size !== b.size) {
    return false;
  }

  for (let item of a) {
    if (!b.has(item)) {
      return false;
    }
  }

  return true;
}
