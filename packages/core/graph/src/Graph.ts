import {fromNodeId} from './types';
import AdjacencyList, {SerializedAdjacencyList} from './AdjacencyList';
import type {Edge, NodeId} from './types';
import type {
  TraversalActions,
  GraphVisitor,
  GraphTraversalCallback,
} from '@atlaspack/types';
import {BitSet} from './BitSet';

import nullthrows from 'nullthrows';

export type NullEdgeType = 1;
export type GraphOpts<TNode, TEdgeType extends number = 1> = {
  nodes?: Array<TNode | null>;
  adjacencyList?: SerializedAdjacencyList<TEdgeType>;
  rootNodeId?: NodeId | null | undefined;
};

export type SerializedGraph<TNode, TEdgeType extends number = 1> = {
  nodes: Array<TNode | null>;
  adjacencyList: SerializedAdjacencyList<TEdgeType>;
  rootNodeId: NodeId | null | undefined;
};

export type AllEdgeTypes = -1;
export const ALL_EDGE_TYPES: AllEdgeTypes = -1;

type DFSCommandVisit<TContext> = {
  nodeId: NodeId;
  context: TContext | null;
};

type DFSCommandExit<TContext> = {
  nodeId: NodeId;
  exit: GraphTraversalCallback<NodeId, TContext>;
  context: TContext | null;
};

/**
 * Internal type used for queue iterative DFS implementation.
 */
type DFSCommand<TContext> =
  | DFSCommandVisit<TContext>
  | DFSCommandExit<TContext>;

/**
 * Options for DFS traversal.
 */
export type DFSParams<TContext> = {
  visit: GraphVisitor<NodeId, TContext>;
  /**
   * Custom function to get next entries to visit.
   *
   * This can be a performance bottleneck as arrays are created on every node visit.
   *
   * @deprecated This will be replaced by a static `traversalType` set of orders in the future
   *
   * Currently, this is only used in 3 ways:
   *
   * - Traversing down the tree (normal DFS)
   * - Traversing up the tree (ancestors)
   * - Filtered version of traversal; which does not need to exist at the DFS level as the visitor
   *   can handle filtering
   * - Sorted traversal of BundleGraph entries, which does not have a clear use-case, but may
   *   not be safe to remove
   *
   * Only due to the latter we aren't replacing this.
   */
  getChildren: (nodeId: NodeId) => Array<NodeId>;
  startNodeId?: NodeId | null | undefined;
};

export default class Graph<TNode, TEdgeType extends number = 1> {
  nodes: Array<TNode | null>;
  adjacencyList: AdjacencyList<TEdgeType>;
  rootNodeId: NodeId | null | undefined;
  _visited: BitSet | null | undefined;

  constructor(opts?: GraphOpts<TNode, TEdgeType> | null) {
    this.nodes = opts?.nodes || [];
    this.setRootNodeId(opts?.rootNodeId);

    let adjacencyList = opts?.adjacencyList;
    this.adjacencyList = adjacencyList
      ? AdjacencyList.deserialize(adjacencyList)
      : new AdjacencyList<TEdgeType>();
  }

  setRootNodeId(id?: NodeId | null) {
    this.rootNodeId = id;
  }

  static deserialize<TNode, TEdgeType extends number = 1>(
    opts: GraphOpts<TNode, TEdgeType>,
  ): Graph<TNode, TEdgeType> {
    return new this({
      nodes: opts.nodes,
      adjacencyList: opts.adjacencyList,
      rootNodeId: opts.rootNodeId,
    });
  }

  serialize(): SerializedGraph<TNode, TEdgeType> {
    return {
      nodes: this.nodes,
      adjacencyList: this.adjacencyList.serialize(),
      rootNodeId: this.rootNodeId,
    };
  }

  // Returns an iterator of all edges in the graph. This can be large, so iterating
  // the complete list can be costly in large graphs. Used when merging graphs.
  getAllEdges(): Iterator<Edge<TEdgeType | NullEdgeType>> {
    return this.adjacencyList.getAllEdges();
  }

  addNode(node: TNode): NodeId {
    let id = this.adjacencyList.addNode();
    this.nodes.push(node);
    return id;
  }

  hasNode(id: NodeId): boolean {
    return this.nodes[id] != null;
  }

  getNode(id: NodeId): TNode | null | undefined {
    return this.nodes[id];
  }

  addEdge(
    from: NodeId,
    to: NodeId,
    type: TEdgeType | NullEdgeType = 1,
  ): boolean {
    if (Number(type) === 0) {
      throw new Error(`Edge type "${type}" not allowed`);
    }

    if (this.getNode(from) == null) {
      throw new Error(`"from" node '${fromNodeId(from)}' not found`);
    }

    if (this.getNode(to) == null) {
      throw new Error(`"to" node '${fromNodeId(to)}' not found`);
    }

    return this.adjacencyList.addEdge(from, to, type);
  }

  hasEdge(
    from: NodeId,
    to: NodeId,
    type: TEdgeType | NullEdgeType | Array<TEdgeType | NullEdgeType> = 1,
  ): boolean {
    return this.adjacencyList.hasEdge(from, to, type);
  }

  getNodeIdsConnectedTo(
    nodeId: NodeId,
    type:
      | TEdgeType
      | NullEdgeType
      | Array<TEdgeType | NullEdgeType>
      | AllEdgeTypes = 1,
  ): Array<NodeId> {
    this._assertHasNodeId(nodeId);

    return this.adjacencyList.getNodeIdsConnectedTo(nodeId, type);
  }

  getNodeIdsConnectedFrom(
    nodeId: NodeId,
    type:
      | TEdgeType
      | NullEdgeType
      | Array<TEdgeType | NullEdgeType>
      | AllEdgeTypes = 1,
  ): Array<NodeId> {
    this._assertHasNodeId(nodeId);

    return this.adjacencyList.getNodeIdsConnectedFrom(nodeId, type);
  }

  // Removes node and any edges coming from or to that node
  removeNode(nodeId: NodeId, removeOrphans: boolean = true) {
    if (!this.hasNode(nodeId)) {
      return;
    }

    for (let {type, from} of this.adjacencyList.getInboundEdgesByType(nodeId)) {
      this._removeEdge(
        from,
        nodeId,
        type,
        // Do not allow orphans to be removed as this node could be one
        // and is already being removed.
        false,
      );
    }

    for (let {type, to} of this.adjacencyList.getOutboundEdgesByType(nodeId)) {
      this._removeEdge(nodeId, to, type, removeOrphans);
    }

    this.nodes[nodeId] = null;
  }

  removeEdges(nodeId: NodeId, type: TEdgeType | NullEdgeType = 1) {
    if (!this.hasNode(nodeId)) {
      return;
    }

    for (let to of this.getNodeIdsConnectedFrom(nodeId, type)) {
      this._removeEdge(nodeId, to, type);
    }
  }

  removeEdge(
    from: NodeId,
    to: NodeId,
    type: TEdgeType | NullEdgeType = 1,
    removeOrphans: boolean = true,
  ) {
    if (!this.adjacencyList.hasEdge(from, to, type)) {
      throw new Error(
        `Edge from ${fromNodeId(from)} to ${fromNodeId(to)} not found!`,
      );
    }

    this._removeEdge(from, to, type, removeOrphans);
  }

  // Removes edge and node the edge is to if the node is orphaned
  _removeEdge(
    from: NodeId,
    to: NodeId,
    type: TEdgeType | NullEdgeType = 1,
    removeOrphans: boolean = true,
  ) {
    if (!this.adjacencyList.hasEdge(from, to, type)) {
      return;
    }

    this.adjacencyList.removeEdge(from, to, type);
    if (removeOrphans && this.isOrphanedNode(to)) {
      this.removeNode(to);
    }
  }

  isOrphanedNode(nodeId: NodeId): boolean {
    if (!this.hasNode(nodeId)) {
      return false;
    }

    if (this.rootNodeId == null) {
      // If the graph does not have a root, and there are inbound edges,
      // this node should not be considered orphaned.
      return !this.adjacencyList.hasInboundEdges(nodeId);
    }

    // Otherwise, attempt to traverse backwards to the root. If there is a path,
    // then this is not an orphaned node.
    let hasPathToRoot = false;
    // go back to traverseAncestors
    this.traverseAncestors(
      nodeId,
      (ancestorId, _, actions) => {
        if (ancestorId === this.rootNodeId) {
          hasPathToRoot = true;
          actions.stop();
        }
      },
      ALL_EDGE_TYPES,
    );

    if (hasPathToRoot) {
      return false;
    }

    return true;
  }

  updateNode(nodeId: NodeId, node: TNode): void {
    this._assertHasNodeId(nodeId);
    this.nodes[nodeId] = node;
  }

  // Update a node's downstream nodes making sure to prune any orphaned branches
  replaceNodeIdsConnectedTo(
    fromNodeId: NodeId,
    toNodeIds: ReadonlyArray<NodeId>,
    replaceFilter?: null | ((arg1: NodeId) => boolean),
    type: TEdgeType | NullEdgeType = 1,
  ): void {
    this._assertHasNodeId(fromNodeId);

    let outboundEdges = this.getNodeIdsConnectedFrom(fromNodeId, type);
    let childrenToRemove = new Set(
      replaceFilter
        ? outboundEdges.filter((toNodeId) => replaceFilter(toNodeId))
        : outboundEdges,
    );
    for (let toNodeId of toNodeIds) {
      childrenToRemove.delete(toNodeId);

      if (!this.hasEdge(fromNodeId, toNodeId, type)) {
        this.addEdge(fromNodeId, toNodeId, type);
      }
    }

    for (let child of childrenToRemove) {
      this._removeEdge(fromNodeId, child, type);
    }
  }

  traverse<TContext>(
    visit: GraphVisitor<NodeId, TContext>,
    startNodeId?: NodeId | null,
    type:
      | TEdgeType
      | NullEdgeType
      | Array<TEdgeType | NullEdgeType>
      | AllEdgeTypes = 1,
  ): TContext | null | undefined {
    let enter = typeof visit === 'function' ? visit : visit.enter;
    if (
      type === ALL_EDGE_TYPES &&
      enter &&
      (typeof visit === 'function' || !visit.exit)
    ) {
      return this.dfsFast(enter, startNodeId);
    } else {
      return this.dfs({
        visit,
        startNodeId,
        getChildren: (nodeId) => this.getNodeIdsConnectedFrom(nodeId, type),
      });
    }
  }

  filteredTraverse<TValue, TContext>(
    filter: (arg1: NodeId, arg2: TraversalActions) => TValue | null | undefined,
    visit: GraphVisitor<TValue, TContext>,
    startNodeId?: NodeId | null,
    type?: TEdgeType | Array<TEdgeType | NullEdgeType> | AllEdgeTypes,
  ): TContext | null | undefined {
    return this.traverse(mapVisitor(filter, visit), startNodeId, type);
  }

  traverseAncestors<TContext>(
    startNodeId: NodeId | null | undefined,
    visit: GraphVisitor<NodeId, TContext>,
    type:
      | TEdgeType
      | NullEdgeType
      | Array<TEdgeType | NullEdgeType>
      | AllEdgeTypes = 1,
  ): TContext | null | undefined {
    return this.dfs({
      visit,
      startNodeId,
      getChildren: (nodeId) => this.getNodeIdsConnectedTo(nodeId, type),
    });
  }

  dfsFast<TContext>(
    visit: GraphTraversalCallback<NodeId, TContext>,
    startNodeId?: NodeId | null,
  ): TContext | null | undefined {
    let traversalStartNode = nullthrows(
      startNodeId ?? this.rootNodeId,
      'A start node is required to traverse',
    );
    this._assertHasNodeId(traversalStartNode);

    // @ts-expect-error - TS7034 - Variable 'visited' implicitly has type 'any' in some locations where its type cannot be determined.
    let visited;
    if (!this._visited || this._visited.capacity < this.nodes.length) {
      this._visited = new BitSet(this.nodes.length);
      visited = this._visited;
    } else {
      visited = this._visited;
      visited.clear();
    }
    // Take shared instance to avoid re-entrancy issues.
    this._visited = null;

    let stopped = false;
    let skipped = false;
    let actions: TraversalActions = {
      skipChildren() {
        skipped = true;
      },
      stop() {
        stopped = true;
      },
    };

    let queue = [{nodeId: traversalStartNode, context: null}];
    while (queue.length !== 0) {
      // @ts-expect-error - TS2339 - Property 'nodeId' does not exist on type '{ nodeId: number; context: null; } | undefined'. | TS2339 - Property 'context' does not exist on type '{ nodeId: number; context: null; } | undefined'.
      let {nodeId, context} = queue.pop();
      if (!this.hasNode(nodeId) || visited.has(nodeId)) continue;
      visited.add(nodeId);

      skipped = false;
      let newContext = visit(nodeId, context, actions);
      if (typeof newContext !== 'undefined') {
        context = newContext;
      }

      if (skipped) {
        continue;
      }

      if (stopped) {
        this._visited = visited;
        return context;
      }

      this.adjacencyList.forEachNodeIdConnectedFromReverse(nodeId, (child) => {
        // @ts-expect-error - TS7005 - Variable 'visited' implicitly has an 'any' type.
        if (!visited.has(child)) {
          queue.push({nodeId: child, context});
        }
        return false;
      });
    }

    this._visited = visited;
    return null;
  }

  // A post-order implementation of dfsFast
  postOrderDfsFast(
    visit: GraphTraversalCallback<NodeId, TraversalActions>,
    startNodeId?: NodeId | null,
  ): void {
    let traversalStartNode = nullthrows(
      startNodeId ?? this.rootNodeId,
      'A start node is required to traverse',
    );
    this._assertHasNodeId(traversalStartNode);

    // @ts-expect-error - TS7034 - Variable 'visited' implicitly has type 'any' in some locations where its type cannot be determined.
    let visited;
    if (!this._visited || this._visited.capacity < this.nodes.length) {
      this._visited = new BitSet(this.nodes.length);
      visited = this._visited;
    } else {
      visited = this._visited;
      visited.clear();
    }
    this._visited = null;

    let stopped = false;
    let actions: TraversalActions = {
      stop() {
        stopped = true;
      },
      skipChildren() {
        throw new Error(
          'Calling skipChildren inside a post-order traversal is not allowed',
        );
      },
    };

    let queue = [traversalStartNode];
    while (queue.length !== 0) {
      let nodeId = queue[queue.length - 1];

      if (!visited.has(nodeId)) {
        visited.add(nodeId);

        this.adjacencyList.forEachNodeIdConnectedFromReverse(
          nodeId,
          (child) => {
            // @ts-expect-error - TS7005 - Variable 'visited' implicitly has an 'any' type.
            if (!visited.has(child)) {
              queue.push(child);
            }
            return false;
          },
        );
      } else {
        queue.pop();
        visit(nodeId, null, actions);

        if (stopped) {
          this._visited = visited;
          return;
        }
      }
    }

    this._visited = visited;
    return;
  }

  /**
   * Iterative implementation of DFS that supports all use-cases.
   *
   * This replaces `dfs` and will replace `dfsFast`.
   */
  dfs<TContext>({
    visit,
    startNodeId,
    getChildren,
  }: DFSParams<TContext>): TContext | null | undefined {
    let traversalStartNode = nullthrows(
      startNodeId ?? this.rootNodeId,
      'A start node is required to traverse',
    );
    this._assertHasNodeId(traversalStartNode);

    let visited;
    if (!this._visited || this._visited.capacity < this.nodes.length) {
      this._visited = new BitSet(this.nodes.length);
      visited = this._visited;
    } else {
      visited = this._visited;
      visited.clear();
    }
    // Take shared instance to avoid re-entrancy issues.
    this._visited = null;

    let stopped = false;
    let skipped = false;
    let actions: TraversalActions = {
      skipChildren() {
        skipped = true;
      },
      stop() {
        stopped = true;
      },
    };

    const queue: DFSCommand<TContext>[] = [
      {nodeId: traversalStartNode, context: null},
    ];
    const enter = typeof visit === 'function' ? visit : visit.enter;
    while (queue.length !== 0) {
      const command = queue.pop();

      // @ts-expect-error - TS2532 - Object is possibly 'undefined'. | TS2339 - Property 'exit' does not exist on type 'DFSCommand<TContext>'.
      if (command.exit != null) {
        // @ts-expect-error - TS2339 - Property 'nodeId' does not exist on type 'DFSCommand<TContext> | undefined'. | TS2339 - Property 'context' does not exist on type 'DFSCommand<TContext> | undefined'. | TS2339 - Property 'exit' does not exist on type 'DFSCommand<TContext> | undefined'.
        let {nodeId, context, exit} = command;
        // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
        let newContext = exit(nodeId, command.context, actions);
        if (typeof newContext !== 'undefined') {
          context = newContext;
        }

        if (skipped) {
          continue;
        }

        if (stopped) {
          this._visited = visited;
          return context;
        }
      } else {
        // @ts-expect-error - TS2339 - Property 'nodeId' does not exist on type 'DFSCommand<TContext> | undefined'. | TS2339 - Property 'context' does not exist on type 'DFSCommand<TContext> | undefined'.
        let {nodeId, context} = command;
        if (!this.hasNode(nodeId) || visited.has(nodeId)) continue;
        visited.add(nodeId);

        skipped = false;
        if (enter) {
          let newContext = enter(nodeId, context, actions);
          if (typeof newContext !== 'undefined') {
            context = newContext;
          }
        }

        if (skipped) {
          continue;
        }

        if (stopped) {
          this._visited = visited;
          return context;
        }

        if (typeof visit !== 'function' && visit.exit) {
          queue.push({
            nodeId,
            exit: visit.exit,
            context,
          });
        }

        // TODO turn into generator function
        const children = getChildren(nodeId);
        for (let i = children.length - 1; i > -1; i -= 1) {
          const child = children[i];
          if (visited.has(child)) {
            continue;
          }

          queue.push({nodeId: child, context});
        }
      }
    }

    this._visited = visited;
  }

  bfs(
    visit: (nodeId: NodeId) => boolean | null | undefined,
  ): NodeId | null | undefined {
    let rootNodeId = nullthrows(
      this.rootNodeId,
      'A root node is required to traverse',
    );

    let queue: Array<NodeId> = [rootNodeId];
    let visited = new Set<NodeId>([rootNodeId]);

    while (queue.length > 0) {
      let node = queue.shift();
      let stop = visit(rootNodeId);
      if (stop === true) {
        return node;
      }

      // @ts-expect-error - TS2345 - Argument of type 'number | undefined' is not assignable to parameter of type 'number'.
      for (let child of this.getNodeIdsConnectedFrom(node)) {
        if (!visited.has(child)) {
          visited.add(child);
          queue.push(child);
        }
      }
    }

    return null;
  }

  topoSort(type?: TEdgeType): Array<NodeId> {
    let sorted: Array<NodeId> = [];
    this.traverse(
      {
        exit: (nodeId) => {
          sorted.push(nodeId);
        },
      },
      null,
      type,
    );
    return sorted.reverse();
  }

  findAncestor(
    nodeId: NodeId,
    fn: (nodeId: NodeId) => boolean,
  ): NodeId | null | undefined {
    let res = null;
    this.traverseAncestors(nodeId, (nodeId, ctx, traversal) => {
      if (fn(nodeId)) {
        res = nodeId;
        traversal.stop();
      }
    });
    return res;
  }

  findAncestors(
    nodeId: NodeId,
    fn: (nodeId: NodeId) => boolean,
  ): Array<NodeId> {
    let res: Array<NodeId> = [];
    this.traverseAncestors(nodeId, (nodeId, ctx, traversal) => {
      if (fn(nodeId)) {
        res.push(nodeId);
        traversal.skipChildren();
      }
    });
    return res;
  }

  findDescendant(
    nodeId: NodeId,
    fn: (nodeId: NodeId) => boolean,
  ): NodeId | null | undefined {
    let res = null;
    this.traverse((nodeId, ctx, traversal) => {
      if (fn(nodeId)) {
        res = nodeId;
        traversal.stop();
      }
    }, nodeId);
    return res;
  }

  findDescendants(
    nodeId: NodeId,
    fn: (nodeId: NodeId) => boolean,
  ): Array<NodeId> {
    let res: Array<NodeId> = [];
    this.traverse((nodeId, ctx, traversal) => {
      if (fn(nodeId)) {
        res.push(nodeId);
        traversal.skipChildren();
      }
    }, nodeId);
    return res;
  }

  _assertHasNodeId(nodeId: NodeId) {
    if (!this.hasNode(nodeId)) {
      throw new Error('Does not have node ' + fromNodeId(nodeId));
    }
  }
}

export function mapVisitor<NodeId, TValue, TContext>(
  filter: (arg1: NodeId, arg2: TraversalActions) => TValue | null | undefined,
  visit: GraphVisitor<TValue, TContext>,
): GraphVisitor<NodeId, TContext> {
  function makeEnter(
    visit: (
      node: TValue,
      context: TContext | null | undefined,
      actions: TraversalActions,
    ) => TContext | null | undefined,
  ) {
    return function mappedEnter(
      nodeId: NodeId,
      context: TContext | null | undefined,
      actions: TraversalActions,
    ) {
      let value = filter(nodeId, actions);
      if (value != null) {
        return visit(value, context, actions);
      }
    };
  }

  if (typeof visit === 'function') {
    return makeEnter(visit);
  }

  let mapped: Record<string, any> = {};
  if (visit.enter != null) {
    mapped.enter = makeEnter(visit.enter);
  }

  if (visit.exit != null) {
    mapped.exit = function mappedExit(
      nodeId: NodeId,
      context: TContext | null | undefined,
      actions: TraversalActions,
    ) {
      let exit = visit.exit;
      if (!exit) {
        return;
      }

      let value = filter(nodeId, actions);
      if (value != null) {
        return exit(value, context, actions);
      }
    };
  }

  return mapped;
}
