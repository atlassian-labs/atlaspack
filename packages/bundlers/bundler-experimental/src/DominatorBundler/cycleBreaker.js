// @flow strict-local

import {hashString} from '@atlaspack/rust';
import type {NodeId} from '@atlaspack/graph';
import nullthrows from 'nullthrows';
import {EdgeContentGraph} from './EdgeContentGraph';
import {findStronglyConnectedComponents} from './cycleBreaker/findStronglyConnectedComponents';

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

    for (let i = 0; i < component.length; i++) {
      const nodeId = component[i];
      const childNode = componentNode.values[i];

      const contentKey = graph.getContentKeyByNodeId(nodeId);
      const id = result.addNodeByContentKey(contentKey, childNode);
      result.addEdge(componentNodeId, id);
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
