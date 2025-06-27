import type {ContentKey, NodeId} from './types';

import Graph, {SerializedGraph, GraphOpts} from './Graph';
import nullthrows from 'nullthrows';

export type ContentGraphOpts<TNode, TEdgeType extends number = 1> = ((GraphOpts<TNode, TEdgeType>) & {
  _contentKeyToNodeId: Map<ContentKey, NodeId>;
  _nodeIdToContentKey: Map<NodeId, ContentKey>;
});
export type SerializedContentGraph<TNode, TEdgeType extends number = 1> = ((SerializedGraph<TNode, TEdgeType>) & {
  _contentKeyToNodeId: Map<ContentKey, NodeId>;
});

export default class ContentGraph<TNode, TEdgeType extends number = 1> extends Graph<TNode, TEdgeType> {
  _contentKeyToNodeId: Map<ContentKey, NodeId>;
  _nodeIdToContentKey: Map<NodeId, ContentKey>;

  constructor(opts?: ContentGraphOpts<TNode, TEdgeType> | null) {
    if (opts) {
      let {_contentKeyToNodeId, _nodeIdToContentKey, ...rest} = opts;
      super(rest);
      this._contentKeyToNodeId = _contentKeyToNodeId;
      this._nodeIdToContentKey = _nodeIdToContentKey;
    } else {
      super();
      this._contentKeyToNodeId = new Map();
      this._nodeIdToContentKey = new Map();
    }
  }

  static deserialize<TNode, TEdgeType extends number = 1>(opts: ContentGraphOpts<TNode, TEdgeType>): ContentGraph<TNode, TEdgeType> {
    return new ContentGraph(opts);
  }

  serialize(): SerializedContentGraph<TNode, TEdgeType> {
    return {
      ...super.serialize(),
      _contentKeyToNodeId: this._contentKeyToNodeId,
      _nodeIdToContentKey: this._nodeIdToContentKey,
    };
  }

  addNodeByContentKey(contentKey: ContentKey, node: TNode): NodeId {
    if (this.hasContentKey(contentKey)) {
      throw new Error('Graph already has content key ' + contentKey);
    }

    let nodeId = super.addNode(node);
    this._contentKeyToNodeId.set(contentKey, nodeId);
    this._nodeIdToContentKey.set(nodeId, contentKey);
    return nodeId;
  }

  addNodeByContentKeyIfNeeded(contentKey: ContentKey, node: TNode): NodeId {
    return this.hasContentKey(contentKey)
      ? this.getNodeIdByContentKey(contentKey)
      : this.addNodeByContentKey(contentKey, node);
  }

  getNodeByContentKey(contentKey: ContentKey): TNode | null | undefined {
    let nodeId = this._contentKeyToNodeId.get(contentKey);
    if (nodeId != null) {
      return super.getNode(nodeId);
    }
  }

  getContentKeyByNodeId(nodeId: NodeId): ContentKey {
    return nullthrows(
      this._nodeIdToContentKey.get(nodeId),
      `Expected node id ${nodeId} to exist`,
    );
  }

  getNodeIdByContentKey(contentKey: ContentKey): NodeId {
    return nullthrows(
      this._contentKeyToNodeId.get(contentKey),
      `Expected content key ${contentKey} to exist`,
    );
  }

  hasContentKey(contentKey: ContentKey): boolean {
    return this._contentKeyToNodeId.has(contentKey);
  }

  removeNode(nodeId: NodeId, removeOrphans: boolean = true): void {
    this._assertHasNodeId(nodeId);
    let contentKey = nullthrows(this._nodeIdToContentKey.get(nodeId));
    this._contentKeyToNodeId.delete(contentKey);
    this._nodeIdToContentKey.delete(nodeId);
    super.removeNode(nodeId, removeOrphans);
  }
}
