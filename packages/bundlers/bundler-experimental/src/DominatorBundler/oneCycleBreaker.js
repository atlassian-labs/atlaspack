// @flow strict-local

import {ContentGraph, type NodeId} from '@atlaspack/graph';
import type {Asset} from '@atlaspack/types';

type StronglyConnectedComponent = NodeId[];

/**
 * Tarjan SCC algorithm
 *
 * https://en.wikipedia.org/wiki/Tarjan%27s_strongly_connected_components_algorithm
 */
export function findStronglyConnectedComponents<T>(
  graph: ContentGraph<T>,
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

  graph.traverse((nodeId) => {
    if (state[nodeId].index == null) {
      strongConnect(nodeId);
    }
  });

  function strongConnect(nodeId) {
    const nodeState = state[nodeId];
    nodeState.index = index;
    nodeState.lowlink = nodeId;

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

    if (nodeState.lowlink === nodeId) {
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
