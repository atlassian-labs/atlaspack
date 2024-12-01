// @flow strict-local

import type {
  Asset,
  Dependency,
  MutableBundleGraph,
  Target,
} from '@atlaspack/types';
import invariant from 'assert';
import {EdgeContentGraph} from './EdgeContentGraph';

export type AssetNode = {|
  type: 'asset',
  id: string,
  asset: Asset,
  entryDependency: Dependency | null,
  target: Target | null,
  /**
   * We mark the assets that are connected to entry dependencies.
   *
   * Later on we will use this information to determine which assets were entry
   * points, as opposed to lifted-up due to being async or chunked.
   */
  isEntryNode: boolean,
  /**
   * This indicates that this was a root node in the original graph.
   *
   * We will use this later because this graph will be transformed into shared
   * bundles / dominator trees.
   */
  isRoot: boolean,
|};

export type SimpleAssetGraphNode = 'root' | AssetNode;

export const simpleAssetGraphEdges = {
  dependency: 1,
  asyncDependency: 2,
};

export type SimpleAssetGraphEdgeWeight = Dependency[];

export type SimpleAssetGraphEdge = $Values<typeof simpleAssetGraphEdges>;

export type SimpleAssetGraph = EdgeContentGraph<
  SimpleAssetGraphNode,
  SimpleAssetGraphEdgeWeight,
>;

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
  const graph = new EdgeContentGraph();

  const rootNodeId = graph.addNodeByContentKey('root', 'root');
  graph.setRootNodeId(rootNodeId);

  const postOrder = [];
  bundleGraph.traverse({
    enter: (node, context) => {
      if (node.type === 'asset') {
        const asset = bundleGraph.getAssetById(node.value.id);
        graph.addNodeByContentKey(node.value.id, {
          id: asset.id,
          type: 'asset',
          asset,
          entryDependency: context?.dependency ?? null,
          target: context?.target ?? null,
          isEntryNode: false,
          isRoot: false,
        });
        return context;
      } else if (node.type === 'dependency' && node.value.isEntry) {
        return {
          dependency: node.value,
          target: node.value.target ?? context?.target,
        };
      } else if (node.type === 'dependency') {
        return null;
      } else {
        return context;
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
    const node = graph.getNode(assetNodeId);
    invariant(node != null && node !== 'root');

    let rootEdges = [];
    for (let dependency of bundleGraph.getIncomingDependencies(childAsset)) {
      if (dependency.isEntry) {
        rootEdges.push(dependency);
        node.isEntryNode = true;
        node.isRoot = true;
      } else if (
        dependency.priority !== 'sync' ||
        (dependency.sourceAssetType != null &&
          dependency.sourceAssetType !== childAsset.type)
      ) {
        rootEdges.push(dependency);
        node.isRoot = true;

        const parentAsset = bundleGraph.getAssetWithDependency(dependency);
        if (!parentAsset) {
          throw new Error('Non entry dependency had no asset');
        }
        graph.addEdge(
          graph.getNodeIdByContentKey(parentAsset.id),
          assetNodeId,
          simpleAssetGraphEdges.asyncDependency,
          // dependency,
        );
      } else {
        const parentAsset = bundleGraph.getAssetWithDependency(dependency);
        if (!parentAsset) {
          throw new Error('Non entry dependency had no asset');
        }
        graph.addEdge(
          graph.getNodeIdByContentKey(parentAsset.id),
          assetNodeId,
          simpleAssetGraphEdges.dependency,
          // dependency,
        );
      }
    }

    if (rootEdges.length) {
      graph.addWeightedEdge(
        rootNodeId,
        assetNodeId,
        simpleAssetGraphEdges.dependency,
        rootEdges,
      );
    }
  }

  return graph;
}
