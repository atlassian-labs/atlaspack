// @flow strict-local

import {hashString} from '@atlaspack/rust';
import type {ContentGraph, NodeId} from '@atlaspack/graph';
import nullthrows from 'nullthrows';
import {EdgeContentGraph} from './EdgeContentGraph';

export type StronglyConnectedComponent = NodeId[];

export type StronglyConnectedComponentNode<T> = {|
  id: string,
  type: 'StronglyConnectedComponent',
  nodeIds: NodeId[],
  values: T[],
|};

export type AcyclicGraph<T, EW> = EdgeContentGraph<
  T | StronglyConnectedComponentNode<T>,
  EW,
>;

export function convertToAcyclicGraph<T, EW>(
  graph: EdgeContentGraph<T, EW>,
): AcyclicGraph<T, EW> {
  const result: EdgeContentGraph<T | StronglyConnectedComponentNode<T>, EW> =
    new EdgeContentGraph();

  const components = findStronglyConnectedComponents(graph).filter(
    (c) => c.length > 1,
  );
  const componentNodes = new Set(components.flatMap((component) => component));
  const componentNodeIdsByNodeId = new Map();

  graph.nodes.forEach((node, nodeId) => {
    if (node != null && !componentNodes.has(nodeId)) {
      result.addNodeByContentKey(graph.getContentKeyByNodeId(nodeId), node);
    } else {
      // Add null node to keep node ids stable
      // $FlowFixMe
      result.addNode(null);
    }
  });

  for (let component of components) {
    const id = `StronglyConnectedComponent:${hashString(component.join(','))}`;
    const componentNode = {
      id,
      type: 'StronglyConnectedComponent',
      nodeIds: component,
      values: component.map((nodeId) => nullthrows(graph.getNode(nodeId))),
    };
    const componentNodeId = result.addNodeByContentKey(id, componentNode);

    for (let nodeId of component) {
      componentNodeIdsByNodeId.set(nodeId, componentNodeId);
    }
  }

  const redirectEdge = (n: NodeId): NodeId =>
    componentNodeIdsByNodeId.get(n) ?? n;

  for (let edge of graph.getAllEdges()) {
    const weight = graph.getEdgeWeight(edge.from, edge.to);
    result.addWeightedEdge(
      redirectEdge(edge.from),
      redirectEdge(edge.to),
      edge.type,
      weight,
    );
  }

  result.setRootNodeId(graph.rootNodeId);

  return result;
}

/**
 * Tarjan SCC algorithm
 *
 * https://en.wikipedia.org/wiki/Tarjan%27s_strongly_connected_components_algorithm
 */
export function findStronglyConnectedComponents<T, E: number>(
  graph: ContentGraph<T, E>,
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
