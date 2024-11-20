// @flow strict-local

import type {Asset, Dependency} from '@atlaspack/types';
import type {MutableBundleGraph} from '@atlaspack/types';
import {ContentGraph} from '@atlaspack/graph';
import invariant from 'assert';

export type AssetNode = {|
  type: 'asset',
  id: string,
  asset: Asset,
  /**
   * This should be a list
   */
  entryDependency: Dependency,
  target: Target,
  /**
   * We mark the assets that are connected to entry dependencies.
   *
   * Later on we will use this information to determine which assets were entry
   * points, as opposed to lifted-up due to being async or chunked.
   */
  isEntryNode: boolean,
|};

export type SimpleAssetGraphNode = 'root' | AssetNode;

export type SimpleAssetGraph = ContentGraph<SimpleAssetGraphNode>;

/**
 * Simplify the BundleGraph structure into a graph that only contains assets
 * with edges representing the dependencies.
 *
 * Put a root node at the top of the graph, linked to the entry assets.
 *
 * All the async boundaries become linked to the root node instead of the
 * assets that depend on them. The same happens if the parent asset is of a
 * different type than the child. For example an HTML file with a JS dependency
 * will be split.
 *
 * Example input:
 * ```
 * digraph root {
 *   page1 -> library1;
 *   page2 -> library2;
 *   library1 -> library3;
 *   library2 -> library3;
 * }
 * ```
 *
 * Example output:
 *
 * ```
 * digraph root {
 *   root -> page1;
 *   root -> page2;
 *   page1 -> library1;
 *   page2 -> library2;
 *   library1 -> library3;
 *   library2 -> library3;
 * }
 * ```
 */
export function bundleGraphToRootedGraph(
  bundleGraph: MutableBundleGraph,
): SimpleAssetGraph {
  const graph = new ContentGraph();

  const rootNodeId = graph.addNodeByContentKey('root', 'root');
  graph.setRootNodeId(rootNodeId);

  const postOrder = [];
  bundleGraph.traverse({
    enter: (node, context) => {
      if (node.type === 'asset') {
        const asset = bundleGraph.getAssetById(node.value.id);
        invariant(context != null);
        graph.addNodeByContentKey(node.value.id, {
          id: asset.id,
          type: 'asset',
          asset,
          entryDependency: context.dependency,
          target: context.target,
          isEntryNode: false,
        });
      } else if (node.type === 'dependency') {
        return {
          dependency: node.value,
          target: node.value.target ?? context?.target,
        };
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
        const node = graph.getNode(assetNodeId);
        invariant(node != null && node !== 'root');
        node.isEntryNode = true;
      } else if (
        dependency.priority !== 'sync' ||
        (dependency.sourceAssetType != null &&
          dependency.sourceAssetType !== childAsset.type)
      ) {
        graph.addEdge(rootNodeId, assetNodeId);

        const parentAsset = bundleGraph.getAssetWithDependency(dependency);
        if (!parentAsset) {
          throw new Error('Non entry dependency had no asset');
        }
        graph.addEdge(
          graph.getNodeIdByContentKey(parentAsset.id),
          assetNodeId,
          2,
        );
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
