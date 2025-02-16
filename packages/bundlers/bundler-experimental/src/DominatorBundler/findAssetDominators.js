// @flow strict-local

import type {MutableBundleGraph} from '@atlaspack/types';
import {type NodeId} from '@atlaspack/graph';
import {bundleGraphToRootedGraph} from './bundleGraphToRootedGraph';
import {convertToAcyclicGraph} from './cycleBreaker';
import type {AcyclicGraph} from './cycleBreaker';
import type {
  AssetNode,
  BundleReferences,
  SimpleAssetGraph,
  SimpleAssetGraphEdgeWeight,
  SimpleAssetGraphNode,
} from './bundleGraphToRootedGraph';
import {EdgeContentGraph} from './EdgeContentGraph';
import {simpleFastDominance} from './findAssetDominators/simpleFastDominance';

export type AssetDominatorTree = AcyclicGraph<
  'root' | AssetNode,
  SimpleAssetGraphEdgeWeight,
>;

/**
 * For all assets, build the dominance relationship of the asset with other
 * assets.
 *
 * This implements "A simple, fast dominance algorithm" - https://www.cs.tufts.edu/comp/150FP/archive/keith-cooper/dom14.pdf
 *
 * The return value contains the **immediate** dominator tree, which is
 * different to the dominator tree.
 */
export function findAssetDominators(bundleGraph: MutableBundleGraph): {|
  dominators: AssetDominatorTree,
  graph: SimpleAssetGraph,
  noCyclesGraph: AcyclicGraph<
    'root' | SimpleAssetGraphNode,
    SimpleAssetGraphEdgeWeight,
  >,
  bundleReferences: BundleReferences,
|} {
  // Build a simpler graph with a root at the top
  const rootedGraph = bundleGraphToRootedGraph(bundleGraph);
  const bundleReferences = rootedGraph.getBundleReferences();
  const simpleAssetGraph = rootedGraph.getGraph();
  const noCyclesGraph = convertToAcyclicGraph(simpleAssetGraph);
  const dominators = simpleFastDominance(noCyclesGraph);
  const dominatorTree = buildDominatorTree(noCyclesGraph, dominators);

  return {
    dominators: dominatorTree,
    graph: simpleAssetGraph,
    noCyclesGraph,
    bundleReferences,
  };
}

/**
 * Given the dominance relationship map `dominators`, build the immediate
 * dominator tree.
 *
 * This is a tree where each node is connected to its immediate dominator.
 */
export function buildDominatorTree<T, EW>(
  graph: EdgeContentGraph<'root' | T, EW>,
  dominators: NodeId[],
): EdgeContentGraph<'root' | T, EW> {
  const dominatorTree = new EdgeContentGraph<'root' | T, EW>();
  const rootId = dominatorTree.addNodeByContentKey('root', 'root');
  dominatorTree.setRootNodeId(rootId);

  graph.traverse((nodeId) => {
    const node = graph.getNode(nodeId);
    const contentKey = graph.getContentKeyByNodeId(nodeId);
    if (node != null && node !== 'root') {
      dominatorTree.addNodeByContentKey(contentKey, node);
    } else if (node == null) {
      // Add null node to keep node ids stable
      // $FlowFixMe
      dominatorTree.addNode(null);
    }
  });

  for (let nodeId = 0; nodeId < dominators.length; nodeId++) {
    const node = graph.getNode(nodeId);
    if (node === 'root' || node == null) {
      continue;
    }

    const contentKey = graph.getContentKeyByNodeId(nodeId);
    const assetDominatorTreeNodeId =
      dominatorTree.getNodeIdByContentKey(contentKey);

    const immediateDominator = dominators[nodeId];
    if (immediateDominator == null) continue;
    const immediateDominatorNode = graph.getNode(immediateDominator);
    if (immediateDominatorNode == null) continue;

    if (immediateDominatorNode === 'root') {
      const edgeWeight = graph.getEdgeWeight(immediateDominator, nodeId);
      dominatorTree.addWeightedEdge(
        rootId,
        assetDominatorTreeNodeId,
        1,
        edgeWeight,
      );
      continue;
    }

    const immediateDominatorId = dominatorTree.getNodeIdByContentKey(
      graph.getContentKeyByNodeId(immediateDominator),
    );
    dominatorTree.addEdge(immediateDominatorId, assetDominatorTreeNodeId);
  }

  return dominatorTree;
}
