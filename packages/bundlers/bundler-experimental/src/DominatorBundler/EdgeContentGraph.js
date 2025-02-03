// @flow strict-local

import {ContentGraph, type NodeId} from '@atlaspack/graph';
import nullthrows from 'nullthrows';

export class EdgeContentGraph<N, EW> extends ContentGraph<N, number> {
  #edgeWeights: Map<string, EW> = new Map();

  clone(): EdgeContentGraph<N, EW> {
    const newGraph: EdgeContentGraph<N, EW> = new EdgeContentGraph();
    let nodeId = 0;
    for (let node of this.nodes) {
      const contentKey = this._nodeIdToContentKey.get(nodeId);
      if (node == null) {
        // Add null node to preserve node ids
        // $FlowFixMe
        newGraph.addNode(null);
      } else if (contentKey == null) {
        newGraph.addNode(node);
      } else {
        newGraph.addNodeByContentKey(contentKey, node);
      }
      nodeId += 1;
    }
    for (let edge of this.getAllEdges()) {
      const weight = this.getEdgeWeight(edge.from, edge.to);
      newGraph.addWeightedEdge(edge.from, edge.to, edge.type, weight);
    }

    if (this.rootNodeId != null) {
      const rootNodeContentKey = this._nodeIdToContentKey.get(this.rootNodeId);
      const newGraphRootNodeId = newGraph.getNodeIdByContentKey(
        nullthrows(rootNodeContentKey),
      );
      newGraph.setRootNodeId(newGraphRootNodeId);
    }

    return newGraph;
  }

  addWeightedEdge(
    from: NodeId,
    to: NodeId,
    type: number,
    weight: EW | null,
  ): void {
    this.addEdge(from, to, type);
    if (weight != null) {
      this.#edgeWeights.set([String(from), String(to)].join(','), weight);
    }
  }

  getEdgeWeight(from: NodeId, to: NodeId): EW | null {
    return this.#edgeWeights.get([String(from), String(to)].join(',')) ?? null;
  }
}
