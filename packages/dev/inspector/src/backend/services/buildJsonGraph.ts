import type {Node} from '@atlaspack/core/lib/types.js';
import {Graph, ALL_EDGE_TYPES} from '@atlaspack/graph';
import {getDisplayName} from './getDisplayName';

export interface JsonGraph<T> {
  nodes: Array<{
    id: string;
    nodeId: string;
    path: string[] | null;
    displayName: string;
    level: number;
    edges: string[];
    extra: T | null;
  }>;
}

/**
 * Build a simplified representation of a graph for a front-end to render.
 *
 * The graph is simplified by:
 * - being a JSON serialisable object
 * - filtering out unnecessary nodes
 * - limiting the depth of nodes returned from the root
 */
export function buildJsonGraph<T>(
  graph: Graph<Node>,
  rootNodeId: string | null,
  filter: (node: Node) => boolean = () => true,
  extra?: (node: Node) => T,
): JsonGraph<T> {
  const depths = new Map<string, number>();

  rootNodeId = rootNodeId
    ? graph.getNodeIdByContentKey(rootNodeId)
    : graph.rootNodeId;

  // build path from `graph.rootNodeId` node to `rootNodeId`
  let path: string[] | null = null;
  {
    const stack: string[] = [];
    const visited = new Set<string>();
    graph.dfs({
      visit: {
        enter(nodeId: string, context: any, actions: {stop: () => void}) {
          stack.push(nodeId);
          visited.add(nodeId);

          if (nodeId === rootNodeId) {
            actions.stop();
            path = stack.slice();
          }
        },
        exit() {
          stack.pop();
        },
      },
      getChildren: (nodeId: string) => {
        return graph.getNodeIdsConnectedFrom(nodeId, ALL_EDGE_TYPES);
      },
      startNodeId: graph.rootNodeId,
    });
  }

  // Build depths of all nodes in the graph
  {
    const topologicalOrder: string[] = [];
    graph.traverse(
      {
        exit(nodeId: string) {
          topologicalOrder.push(nodeId);
        },
      },
      rootNodeId,
      ALL_EDGE_TYPES,
    );
    topologicalOrder.reverse();

    for (let i = 0; i < topologicalOrder.length; i++) {
      const nodeId = topologicalOrder[i];
      const currentDepth = depths.get(nodeId) ?? 0;
      const neighbors = graph.getNodeIdsConnectedFrom(nodeId, ALL_EDGE_TYPES);
      for (const other of neighbors) {
        const increment = filter(graph.getNode(other)) ? 1 : 0;
        depths.set(
          other,
          Math.max(currentDepth + increment, depths.get(other) ?? 0),
        );
      }
    }
  }

  const allNodes: string[] = [];
  graph.traverse(
    (nodeId: string) => {
      allNodes.push(nodeId);
    },
    rootNodeId,
    ALL_EDGE_TYPES,
  );

  const maxDepth = 100;
  return {
    nodes: allNodes
      .filter((nodeId) => (depths.get(nodeId) ?? 0) < maxDepth)
      .filter((nodeId) => filter(graph.getNode(nodeId)))
      .map((nodeId) => ({
        id: graph.getNode(nodeId).id,
        nodeId,
        path:
          nodeId === rootNodeId
            ? path?.map((id) => graph.getNode(id).id) ?? null
            : null,
        displayName: getDisplayName(graph.getNode(nodeId)),
        level: depths.get(nodeId) ?? 0,
        edges: graph
          .getNodeIdsConnectedFrom(nodeId, ALL_EDGE_TYPES)
          .map((nodeId) => graph.getNode(nodeId).id),
        extra: extra ? extra(graph.getNode(nodeId)) : null,
      })),
  };
}
