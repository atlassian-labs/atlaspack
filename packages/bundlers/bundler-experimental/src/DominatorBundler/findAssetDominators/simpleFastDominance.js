// @flow strict-local

import {type NodeId, Graph, ALL_EDGE_TYPES} from '@atlaspack/graph';

/**
 * Implements "A simple, fast dominance algorithm", to find the immediate
 * dominators of all nodes in a graph.
 *
 * Returns a map of node IDs to their immediate dominator's node ID.
 * This map is represented by an array where the index is the node ID and the
 * value is its dominator.
 *
 * For example, given a node `3`, `dominators[3]` is the immediate dominator
 * of node 3.
 *
 * - https://www.cs.tufts.edu/comp/150FP/archive/keith-cooper/dom14.pdf
 */
export function simpleFastDominance<T>(graph: Graph<T, number>): NodeId[] {
  const rootNodeId = graph.rootNodeId;
  if (rootNodeId == null) {
    throw new Error('Graph must have a root node');
  }

  const postOrder = getGraphPostOrder(graph);
  const reversedPostOrder = postOrder.slice().reverse();
  const dominators = Array(graph.nodes.length).fill(null);

  const postOrderIndexes = Array(graph.nodes.length).fill(null);
  for (let i = 0; i < postOrder.length; i++) {
    postOrderIndexes[postOrder[i]] = i;
  }

  dominators[rootNodeId] = graph.rootNodeId;

  let changed = true;

  while (changed) {
    changed = false;

    for (let node of reversedPostOrder) {
      if (node === graph.rootNodeId) continue;

      let newImmediateDominator = null;
      graph.forEachNodeIdConnectedTo(
        node,
        (predecessor) => {
          if (newImmediateDominator == null) {
            newImmediateDominator = predecessor;
          } else {
            if (dominators[predecessor] == null) {
              return;
            }

            newImmediateDominator = intersect(
              postOrderIndexes,
              dominators,
              predecessor,
              newImmediateDominator,
            );
          }
        },
        ALL_EDGE_TYPES,
      );

      if (dominators[node] !== newImmediateDominator) {
        dominators[node] = newImmediateDominator;
        changed = true;
      }
    }
  }

  return dominators;
}

/**
 * Return the post-order of the graph.
 */
export function getGraphPostOrder<T>(
  graph: Graph<T, number>,
  type: number = 1,
): NodeId[] {
  const postOrder = [];
  graph.traverse(
    {
      exit: (node) => {
        postOrder.push(node);
      },
    },
    graph.rootNodeId,
    type,
  );
  return postOrder;
}

/**
 * From "A Simple, Fast Dominance Algorithm"
 * Keith D. Cooper, Timothy J. Harvey, and Ken Kennedy:
 *
 * > The intersection routine appears at the bottom of the figure.
 * > It implements a “two-finger” algorithm – one can imagine a finger pointing
 * > to each dominator set, each finger moving independently as the comparisons
 * > dictate. In this case, the comparisons are on postorder numbers; for each
 * > intersection, we start the two fingers at the ends of the two sets, and,
 * > until the fingers point to the same postorder number, we move the finger
 * > pointing to the smaller number back one element. Remember that nodes higher
 * > in the dominator tree have higher postorder numbers, which is why intersect
 * > moves the finger whose value is less than the other finger’s. When the two
 * > fingers point at the same element, intersect returns that element. The set
 * > resulting from the intersection begins with the returned element and chains
 * > its way up the doms array to the entry node.
 *
 * `postOrder` is the post-order node list of the graph.
 *
 * `dominators` is the current immediate dominator state for node in the graph.
 *
 * This is coupled with the fact node ids are indexes into an array. It is a map
 * of NodeId -> NodeId, where the value at index `i` is the immediate dominator
 * of the node `i`.
 *
 * `predecessor` is one predecessor node id of the node we're currently
 * computing the immediate dominator for.
 *
 * `newImmediateDominator` is current best immediate dominator candidate for the
 * node we're computing the immediate dominator for.
 *
 * The algorithm is intersecting the dominator sets of the two predecessors and
 * returning dominator node with the highest post-order number by walking up
 * the dominator tree until the two sets intersect.
 *
 * The node with the highest post-order index is the immediate dominator, as
 * it is the closest to the node we're computing for.
 */
export function intersect(
  postOrderIndexes: number[],
  dominators: (NodeId | null)[],
  predecessor: NodeId,
  newImmediateDominator: NodeId,
): NodeId {
  let n1: number = predecessor;
  let n2: number = newImmediateDominator;
  while (n1 !== n2) {
    while (postOrderIndexes[n1] < postOrderIndexes[n2]) {
      n1 = Number(dominators[n1]);
    }
    while (postOrderIndexes[n2] < postOrderIndexes[n1]) {
      n2 = Number(dominators[n2]);
    }
  }
  return n1;
}
