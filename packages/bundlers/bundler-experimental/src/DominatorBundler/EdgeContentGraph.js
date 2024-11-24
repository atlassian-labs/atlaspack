// @flow strict-local

import {ContentGraph, type NodeId} from '@atlaspack/graph';
import nullthrows from 'nullthrows';

export class EdgeContentGraph<N, EW, E: number = 1> extends ContentGraph<N, E> {
  #edgeWeights: Map<[NodeId, NodeId], EW> = new Map();

  clone(): EdgeContentGraph<TNode, TEdgeType> {
    const newGraph = new EdgeContentGraph();
    let nodeId = 0;
    for (let node of this.nodes) {
      if (node == null) continue;
      const contentKey = this._nodeIdToContentKey.get(nodeId);
      if (contentKey == null) {
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

  addWeightedEdge(from: NodeId, to: NodeId, type: E, weight: EW): void {
    this.addEdge(from, to, type);
    this.#edgeWeights.set([String(from), String(to)].join(','), weight);
  }

  getEdgeWeight(from: NodeId, to: NodeId): EW {
    return this.#edgeWeights.get([String(from), String(to)].join(','));
  }
}
