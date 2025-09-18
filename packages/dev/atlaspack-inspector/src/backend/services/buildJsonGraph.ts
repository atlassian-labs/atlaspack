/* eslint-disable import/no-extraneous-dependencies */
/* eslint-disable monorepo/no-internal-import */
// @ts-expect-error TS2749
import type {Node} from '@atlaspack/core/lib/types.js';
import {ALL_EDGE_TYPES, ContentGraph} from '@atlaspack/graph';
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
  graph: ContentGraph<Node>,
  rootNodeId: string | null | undefined,
  filter: (node: Node) => boolean = () => true,
  extra?: (node: Node) => T,
): JsonGraph<T> {
  const depths = new Map<number, number>();

  const rootNodeIndex = rootNodeId
    ? graph.getNodeIdByContentKey(rootNodeId)
    : graph.rootNodeId;

  // build path from `graph.rootNodeId` node to `rootNodeId`
  let path: number[] | null = null;
  {
    const stack: number[] = [];
    const visited = new Set<number>();
    graph.dfs({
      visit: {
        enter(nodeId: number, context: any, actions: {stop: () => void}) {
          stack.push(nodeId);
          visited.add(nodeId);

          if (nodeId === rootNodeIndex) {
            actions.stop();
            path = stack.slice();
          }
        },
        exit() {
          stack.pop();
        },
      },
      getChildren: (nodeId: number) => {
        return graph.getNodeIdsConnectedFrom(nodeId, ALL_EDGE_TYPES);
      },
      startNodeId: graph.rootNodeId,
    });
  }

  // Build depths of all nodes in the graph
  {
    const topologicalOrder: number[] = [];
    graph.traverse(
      {
        exit(nodeId: number) {
          topologicalOrder.push(nodeId);
        },
      },
      rootNodeIndex,
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
    (nodeId: any) => {
      allNodes.push(nodeId);
    },
    rootNodeIndex,
    ALL_EDGE_TYPES,
  );

  const maxDepth = 100;
  return {
    nodes: allNodes
      .filter((nodeId: any) => (depths.get(nodeId) ?? 0) < maxDepth)
      .filter((nodeId: any) => filter(graph.getNode(nodeId)))
      .map((nodeId: any) => ({
        id: graph.getNode(nodeId).id,
        nodeId,
        path:
          nodeId === rootNodeIndex
            ? (path?.map((id: any) => graph.getNode(id).id) ?? null)
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
