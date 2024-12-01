// @flow strict-local

import type {AssetNode, SimpleAssetGraph} from './bundleGraphToRootedGraph';
import {getGraphPostOrder} from './findAssetDominators';
import type {NodeId} from '@atlaspack/graph';

export type NodeEntryDependencies = {|
  entryDependenciesByAsset: Map<NodeId, Set<AssetNode>>,
  asyncDependenciesByAsset: Map<NodeId, Set<AssetNode>>,
|};

/**
 * Traverses the graph in post-order, to find for any given asset, the
 * sets of entry dependencies and async boundaries through which it is
 * reachable.
 */
export function findNodeEntryDependencies(
  graph: SimpleAssetGraph,
): NodeEntryDependencies {
  const postOrder = getGraphPostOrder(graph, -1);
  const reversePostOrder = [...postOrder].reverse();

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

    const entryNode = entryDependencyNodes.get(nodeId)?.node;
    if (entryNode != null) {
      const dependencies = entryDependenciesByAsset.get(nodeId) ?? new Set();
      dependencies.add(entryNode);
      entryDependenciesByAsset.set(nodeId, dependencies);
    }

    const asyncNode = asyncDependencyNodes.get(nodeId)?.node;
    if (asyncNode != null) {
      const dependencies = asyncDependenciesByAsset.get(nodeId) ?? new Set();
      dependencies.add(asyncNode);
      asyncDependenciesByAsset.set(nodeId, dependencies);
    }

    graph.forEachNodeIdConnectedTo(nodeId, (parentId) => {
      const parent = graph.getNode(parentId);
      if (parent == null || parent === 'root') {
        return;
      }

      const parentEntryDependency = parent.entryDependency;
      if (parentEntryDependency != null) {
        const dependencies = entryDependenciesByAsset.get(nodeId) ?? new Set();
        dependencies.add(parent);
        entryDependenciesByAsset.set(nodeId, dependencies);
      }

      const parentEntries = entryDependenciesByAsset.get(parentId);
      if (parentEntries != null) {
        const dependencies = entryDependenciesByAsset.get(nodeId) ?? new Set();
        for (let parentEntry of parentEntries) {
          dependencies.add(parentEntry);
        }
        entryDependenciesByAsset.set(nodeId, dependencies);
      }

      const parentAsyncEntries = asyncDependenciesByAsset.get(parentId);
      if (parentAsyncEntries != null) {
        const dependencies = asyncDependenciesByAsset.get(nodeId) ?? new Set();
        for (let asyncNode of parentAsyncEntries) {
          dependencies.add(asyncNode);
        }
        asyncDependenciesByAsset.set(nodeId, dependencies);
      }
    });
  }

  return {
    entryDependenciesByAsset,
    asyncDependenciesByAsset,
  };
}
