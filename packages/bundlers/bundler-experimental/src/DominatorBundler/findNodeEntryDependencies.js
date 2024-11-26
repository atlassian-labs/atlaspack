// @flow strict-local

import type {AssetNode, SimpleAssetGraph} from './bundleGraphToRootedGraph';
import {getGraphPostOrder} from './findAssetDominators';
import {NodeId} from '@atlaspack/graph';

/**
 * Traverses the graph in post-order, to find for any given asset, the
 * sets of entry dependencies and async boundaries through which it is
 * reachable.
 */
export function findNodeEntryDependencies(graph: SimpleAssetGraph): {|
  entryDependenciesByAsset: Map<NodeId, Set<AssetNode>>,
  asyncDependenciesByAsset: Map<NodeId, Set<AssetNode>>,
|} {
  const postOrder = getGraphPostOrder(graph);
  const reversePostOrder = postOrder.slice(0).reverse();

  const entryDependenciesByAsset = new Map();
  const asyncDependenciesByAsset = new Map();

  const topLevelNodes = graph.getNodeIdsConnectedFrom(
    graph.getNodeIdByContentKey('root'),
  );

  const entryDependencyNodes = new Map();
  const asyncDependencyNodes = new Map();
  topLevelNodes.forEach((id) => {
    const node = graph.getNode(id);
    if (node == null || node === 'root') {
      return;
    }

    const entryDependency = node.entryDependency;
    if (entryDependency != null) {
      entryDependencyNodes.set(id, {id, node, entryDependency});
    } else {
      asyncDependencyNodes.set(id, {id, node});
    }
  });

  for (let nodeId of reversePostOrder) {
    const node = graph.getNode(nodeId);
    if (node == null || node === 'root') {
      continue;
    }

    graph.forEachNodeIdConnectedTo(nodeId, (parentId) => {
      const parent = graph.getNode(parentId);
      if (parent == null || parent === 'root') {
        entryDependenciesByAsset.set(nodeId, new Set());
        asyncDependenciesByAsset.set(nodeId, new Set());
        return;
      }

      const parentEntryDependency = parent.entryDependency;
      if (parentEntryDependency != null) {
        const entryDependencies =
          entryDependenciesByAsset.get(nodeId) ?? new Set();
        entryDependencies.add(parent);
        entryDependenciesByAsset.set(nodeId, entryDependencies);
      }

      if (entryDependencyNodes.has(parentId)) {
        const parentDependencies =
          entryDependenciesByAsset.get(nodeId) ?? new Set();
        parentDependencies.add(entryDependencyNodes.get(parentId).node);
        entryDependenciesByAsset.set(nodeId, parentDependencies);
      }

      if (asyncDependencyNodes.has(parentId)) {
        const asyncDependencies =
          asyncDependenciesByAsset.get(nodeId) ?? new Set();
        asyncDependencies.add(asyncDependencyNodes.get(parentId).node);
        asyncDependenciesByAsset.set(nodeId, asyncDependencies);
      }
    });
  }

  return {
    entryDependenciesByAsset,
    asyncDependenciesByAsset,
  };
}
