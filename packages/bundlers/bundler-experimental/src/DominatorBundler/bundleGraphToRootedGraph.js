// @flow strict-local

import type {Asset} from '@atlaspack/types';
import type {MutableBundleGraph} from '@atlaspack/types';
import {ContentGraph} from '@atlaspack/graph';

/**
 * Simplify the BundleGraph structure into a graph that only contains assets
 * with edges representing the dependencies.
 *
 * Put a root node at the top of the graph, linked to the entry assets.
 *
 * Example:
 *
 * ```
 * digraph root {
 *   root -> page1
 *   root -> page2
 *   page1 -> library1
 *   page2 -> library2
 *   library1 -> library3
 *   library2 -> library3
 * }
 * ```
 */
export function bundleGraphToRootedGraph(
  bundleGraph: MutableBundleGraph,
): ContentGraph<'root' | Asset> {
  const graph = new ContentGraph();

  const rootNodeId = graph.addNodeByContentKey('root', 'root');
  graph.setRootNodeId(rootNodeId);

  const postOrder = [];
  bundleGraph.traverse({
    enter: (node) => {
      if (node.type === 'asset') {
        const asset = bundleGraph.getAssetById(node.value.id);
        graph.addNodeByContentKey(node.value.id, asset);
      }
    },
    exit: (node) => {
      if (node.type === 'asset') {
        postOrder.push(node.value.id);
      }
    },
  });
  const reversedPostOrder = postOrder.slice().reverse();

  for (let assetId of reversedPostOrder) {
    const childAsset = bundleGraph.getAssetById(assetId);
    const assetNodeId = graph.getNodeIdByContentKey(assetId);

    for (let dependency of bundleGraph.getIncomingDependencies(childAsset)) {
      if (dependency.isEntry) {
        graph.addEdge(rootNodeId, assetNodeId);
      } else {
        const parentAsset = bundleGraph.getAssetWithDependency(dependency);
        if (!parentAsset) {
          throw new Error('Non entry dependency had no asset');
        }
        graph.addEdge(graph.getNodeIdByContentKey(parentAsset.id), assetNodeId);
      }
    }
  }

  return graph;
}