// @flow strict-local

import type {Graph, NodeId} from '@atlaspack/graph';

export type StronglyConnectedComponent = NodeId[];

/**
 * Robert Tarjan's algorithm to find strongly connected components in a graph.
 *
 * Time complexity: O(V + E)
 * Space complexity (worst case): O(V)
 *
 * * https://web.archive.org/web/20170829214726id_/http://www.cs.ucsb.edu/~gilbert/cs240a/old/cs240aSpr2011/slides/TarjanDFS.pdf
 * * https://en.wikipedia.org/wiki/Tarjan%27s_strongly_connected_components_algorithm
 */
export function findStronglyConnectedComponents<T, E: number>(
  graph: Graph<T, E>,
): StronglyConnectedComponent[] {
  type State = {|
    index: null | NodeId,
    lowlink: null | NodeId,
    onStack: boolean,
  |};
  const result = [];
  let index = 0;
  const stack = [];
  const state: State[] = new Array(graph.nodes.length).fill(null).map(() => ({
    index: null,
    lowlink: null,
    onStack: false,
  }));
  graph.nodes.forEach((node, nodeId) => {
    if (node != null && state[nodeId].index == null) {
      strongConnect(nodeId);
    }
  });
  function strongConnect(nodeId) {
    const nodeState = state[nodeId];
    nodeState.index = index;
    nodeState.lowlink = index;

    index += 1;

    stack.push(nodeId);
    nodeState.onStack = true;

    for (let neighborId of graph.getNodeIdsConnectedFrom(nodeId)) {
      const neighborState = state[neighborId];
      if (neighborState.index == null) {
        strongConnect(neighborId);

        nodeState.lowlink = Math.min(
          Number(nodeState.lowlink),
          Number(neighborState.lowlink),
        );
      } else if (neighborState.onStack) {
        nodeState.lowlink = Math.min(
          Number(nodeState.lowlink),
          Number(neighborState.index),
        );
      }
    }

    if (nodeState.lowlink === nodeState.index) {
      const component = [];
      let member;

      do {
        member = stack.pop();
        state[member].onStack = false;
        component.push(member);
      } while (member !== nodeId);

      result.push(component);
    }
  }

  return result;
}
