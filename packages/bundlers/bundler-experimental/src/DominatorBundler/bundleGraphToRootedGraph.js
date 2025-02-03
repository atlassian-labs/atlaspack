// @flow strict-local

import type {
  Asset,
  Dependency,
  MutableBundleGraph,
  Target,
} from '@atlaspack/types';
import type {NodeId} from '@atlaspack/graph';
import invariant from 'assert';
import {EdgeContentGraph} from './EdgeContentGraph';

export type AssetNode = {|
  type: 'asset',
  id: string,
  /**
   * The asset that this node represents.
   */
  asset: Asset,
  /**
   * If this node has a synchronous loading dependency link to an entry-point,
   * this is the entrypoint's dependency.
   */
  entryDependency: Dependency | null,
  /**
   * If this node has a synchronous loading dependency link to an entry-point,
   * this is the entrypoint's target.
   */
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

export type BundleReferences = Map<
  string,
  {|assetNodeId: NodeId, dependency: Dependency, assetContentKey: string|}[],
>;

export class RootedAssetGraph {
  #graph: SimpleAssetGraph;
  #rootNodeId: NodeId;
  #bundleReferences: BundleReferences = new Map();

  constructor() {
    this.#graph = new EdgeContentGraph();
    this.#rootNodeId = this.#graph.addNodeByContentKey('root', 'root');
    this.#graph.setRootNodeId(this.#rootNodeId);
  }

  getRootNodeId(): NodeId {
    return this.#rootNodeId;
  }

  getGraph(): EdgeContentGraph<
    SimpleAssetGraphNode,
    SimpleAssetGraphEdgeWeight,
  > {
    return this.#graph;
  }

  getBundleReferences(): BundleReferences {
    return this.#bundleReferences;
  }

  trackBundleReference(
    parentNodeId: NodeId,
    assetNodeId: NodeId,
    dependency: Dependency,
  ) {
    if (dependency.priority !== 'lazy') {
      const parentContentKey = this.#graph.getContentKeyByNodeId(parentNodeId);
      const bundleReferences =
        this.#bundleReferences.get(parentContentKey) ?? [];
      const assetContentKey = this.#graph.getContentKeyByNodeId(assetNodeId);
      bundleReferences.push({assetNodeId, assetContentKey, dependency});
      this.#bundleReferences.set(parentNodeId, bundleReferences);
    }
  }
}

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
): RootedAssetGraph {
  const result = new RootedAssetGraph();
  const graph = result.getGraph();

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
      } else if (node.type === 'dependency' && node.value.priority === 'lazy') {
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
      } else {
        if (dependency.priority !== 'sync') {
          rootEdges.push(dependency);
          node.isRoot = true;
        } else if (dependency.sourceAssetType !== childAsset.type) {
          rootEdges.push(dependency);
          node.isRoot = true;
        }

        const parentAsset = bundleGraph.getAssetWithDependency(dependency);
        if (!parentAsset) {
          throw new Error(
            `Non entry dependency had no asset: ${dependency.id} - ${dependency.specifier}`,
          );
        }

        const parentNodeId = graph.getNodeIdByContentKey(parentAsset.id);
        const weights = graph.getEdgeWeight(parentNodeId, assetNodeId) ?? [];
        weights.push(dependency);

        graph.addWeightedEdge(
          parentNodeId,
          assetNodeId,
          dependency.priority === 'lazy'
            ? simpleAssetGraphEdges.asyncDependency
            : simpleAssetGraphEdges.dependency,
          weights,
        );
        result.trackBundleReference(parentNodeId, assetNodeId, dependency);
      }
    }

    if (rootEdges.length) {
      graph.addWeightedEdge(
        result.getRootNodeId(),
        assetNodeId,
        simpleAssetGraphEdges.dependency,
        rootEdges,
      );
    }
  }

  return result;
}
