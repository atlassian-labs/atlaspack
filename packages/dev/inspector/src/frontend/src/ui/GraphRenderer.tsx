import React, {useMemo, useRef} from 'react';
import {Link} from 'react-router';
import {ArrowRenderer} from './ArrowRenderer';
import {Graph, Node} from './Graph';
import {GraphNode} from './GraphNode';

export function GraphRenderer({
  graph,
  graphType,
}: {
  graph: Graph;
  graphType: string;
}) {
  const nodeRefs = useRef<Map<string, React.RefObject<HTMLDivElement>>>(
    new Map(),
  );

  const connections = useMemo(() => {
    if (!graph) return [];

    const newConnections: {from: string; to: string}[] = [];
    graph.nodes.forEach((node) => {
      for (const toId of node.edges) {
        newConnections.push({
          from: node.id,
          to: toId,
        });
      }
    });
    return newConnections;
  }, [graph]);

  const levels = useMemo(() => {
    let maxLevel = 0;
    graph.nodes.forEach((node) => {
      maxLevel = Math.max(maxLevel, node.level);
    });

    const result: Node[][] = [];
    for (let i = 0; i <= maxLevel; i++) {
      const nodes = graph.nodes.filter((node) => node.level === i);
      if (nodes.length === 0) continue;
      result.push(nodes);
    }
    return result;
  }, [graph]);

  const nodeEls = levels.map((level, i) => (
    <div
      key={i}
      style={{display: 'flex', flexDirection: 'column', gap: '10px'}}
    >
      {level.map((node) => (
        <GraphNode
          key={node.nodeId}
          node={node}
          nodeRefs={nodeRefs}
          graphType={graphType}
        />
      ))}
    </div>
  ));

  return (
    <>
      {graph.nodes[0]?.path && (
        <div
          style={{
            display: 'flex',
            gap: '10px',
            alignItems: 'center',
          }}
        >
          {graph.nodes[0]?.path.map((id, i) => (
            <React.Fragment key={i}>
              <div>
                <Link to={`?rootNodeId=${id}`}>
                  <pre>{id}</pre>
                </Link>
              </div>

              {i < (graph.nodes[0].path?.length ?? 0) - 1 && <div>&gt;</div>}
            </React.Fragment>
          ))}
        </div>
      )}

      <div
        style={{
          position: 'relative',
          overflow: 'auto',
          minHeight: '100%',
          display: 'flex',
          gap: '20px',
          flex: 1,
        }}
      >
        {nodeEls}
        <ArrowRenderer connections={connections} nodeRefs={nodeRefs} />
      </div>
    </>
  );
}
