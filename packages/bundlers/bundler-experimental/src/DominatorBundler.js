// @flow strict-local

import type {BundleGraph, Bundle, Asset, Dependency} from '@atlaspack/types';
import MutableBundleGraph from '@atlaspack/core/src/public/MutableBundleGraph';
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

export function bundleGraphToRootedGraph(
  bundleGraph: MutableBundleGraph,
): ContentGraph<'root' | Asset> {
  const graph = new ContentGraph();

  const rootNodeId = graph.addNodeByContentKey('root', 'root');
  graph.setRootNodeId(rootNodeId);

  const postOrder = [];
  bundleGraph.traverse({
    exit: (node) => {
      if (node.type === 'asset') {
        postOrder.push(node.value.id);
      }
    },
  });
  const reversedPostOrder = postOrder.slice().reverse();

  for (let assetId of reversedPostOrder) {
    const childAsset = bundleGraph.getAssetById(assetId);
    const assetNodeId = graph.addNodeByContentKey(assetId, childAsset);

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

/**
 * For all assets, build the dominance relationship of the asset with other
 * assets.
 *
 * This implements "A simple, fast dominance algorithm" - https://www.cs.tufts.edu/comp/150FP/archive/keith-cooper/dom14.pdf
 *
 * The return value contains the **immediate** dominator tree, which is
 * different to the dominator tree.
 */
export function findAssetDominators(
  bundleGraph: MutableBundleGraph,
): Map<Asset, Set<Asset>> {
  // Build a simpler graph with a root at the top
  const graph = bundleGraphToRootedGraph(bundleGraph);

  const postOrder = [];
  graph.traverse({
    exit: (node) => {
      postOrder.push(node);
    },
  });
  const reversedPostOrder = postOrder.slice().reverse();

  const dominators = Array(graph.nodes.length).fill(null);
  // we know there's a root node
  dominators[graph.rootNodeId ?? 0] = graph.rootNodeId;

  let changed = true;

  while (changed) {
    changed = false;

    for (let node of reversedPostOrder) {
      if (node === graph.rootNodeId) continue;

      const predecessors = graph.getNodeIdsConnectedTo(node);

      let newImmediateDominator = predecessors[0];
      for (let i = 1; i < predecessors.length; i++) {
        const predecessor = predecessors[i];
        if (dominators[predecessor] == null) {
          continue;
        }

        newImmediateDominator = intersect(
          postOrder,
          dominators,
          predecessor,
          newImmediateDominator,
        );
      }

      if (dominators[node] !== newImmediateDominator) {
        dominators[node] = newImmediateDominator;
        changed = true;
      }
    }
  }

  const outputMap: Map<Asset, Set<Asset>> = new Map();
  bundleGraph.traverse((node) => {
    if (node.type === 'asset') {
      outputMap.set(node.value, new Set());
    }
  });

  for (let nodeId = 0; nodeId < dominators.length; nodeId++) {
    const asset = graph.getNode(nodeId);
    if (asset === 'root' || asset == null) {
      continue;
    }
    const immediateDominator = dominators[nodeId];
    if (immediateDominator == null) continue;
    const immediateDominatorNode = graph.getNode(immediateDominator);
    if (immediateDominatorNode == null || immediateDominatorNode === 'root')
      continue;

    let dominatorSet = outputMap.get(asset);
    dominatorSet?.add(immediateDominatorNode);
  }

  return outputMap;
}

function intersect(
  postOrder: number[],
  dominators: (number | null)[],
  predecessor: number,
  newImmediateDominator: number,
) {
  let n1: number = predecessor;
  let n2: number = newImmediateDominator;
  while (n1 !== n2) {
    while (postOrder.indexOf(n1) < postOrder.indexOf(n2)) {
      n1 = Number(dominators[n1]);
    }
    while (postOrder.indexOf(n2) < postOrder.indexOf(n1)) {
      n2 = Number(dominators[n2]);
    }
  }
  return n1;
}

export function createPackages(
  bundleGraph: MutableBundleGraph,
  dominators: Map<Asset, Set<Asset>>,
) {
  // turn the dominator map into a graph instance
  const graph = new ContentGraph();
  for (let [root, assets] of dominators) {
    const chunk = {
      id: `chunk:${root.id}`,
      assets: new Set([root, ...assets]),
    };
    graph.addNodeByContentKey(chunk.id, chunk);
  }
  for (let [root] of dominators) {
    const chunkId = `chunk:${root.id}`;
    const dependencies = bundleGraph.getDependencies(root);
    for (let dependency of dependencies) {
      const parentAsset = bundleGraph.getAssetWithDependency(dependency);
      if (!parentAsset) {
        throw new Error('Non entry dependency had no asset');
      }
      const parentChunkId = `chunk:${parentAsset.id}`;
      graph.addEdge(
        graph.getNodeIdByContentKey(chunkId),
        graph.getNodeIdByContentKey(parentChunkId),
      );
    }
  }

  // now we need to topo sort and create packages based on their parent IDs
  // if we just iterated the map in any order, we'd not converge
  // TODO
}

export function isSetEqual(a: Set<Asset>, b: Set<Asset>): boolean {
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
