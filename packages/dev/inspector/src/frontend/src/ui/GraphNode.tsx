import React, {useRef, useEffect, useState} from 'react';
import {Link} from 'react-router';
import {Node} from './Graph';
import {NodeDetailsTooltip} from './NodeDetailsTooltip';

export function GraphNode({
  node,
  nodeRefs,
  graphType,
}: {
  node: Node;
  nodeRefs: React.RefObject<Map<string, React.RefObject<HTMLDivElement>>>;
  graphType: string;
}) {
  const [isHovered, setIsHovered] = useState(false);
  const nodeRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (nodeRefs.current) {
      nodeRefs.current.set(node.id, nodeRef);
    }
  }, [node.id, nodeRefs]);

  return (
    <>
      <div
        ref={nodeRef}
        onMouseEnter={() => {
          setIsHovered(true);
        }}
        onMouseLeave={() => {
          setIsHovered(false);
        }}
        style={{
          padding: '10px',
          // margin: '10px',
          border: '1px solid #ccc',
          borderRadius: '4px',
          backgroundColor: '#fff',
          position: 'relative',
          // width: '',
        }}
      >
        <Link to={`?rootNodeId=${encodeURIComponent(node.id)}`}>
          <pre>{node.displayName}</pre>
        </Link>
      </div>

      {isHovered && (
        <NodeDetailsTooltip
          nodeRef={nodeRef}
          node={node}
          graphType={graphType}
        />
      )}
    </>
  );
}
