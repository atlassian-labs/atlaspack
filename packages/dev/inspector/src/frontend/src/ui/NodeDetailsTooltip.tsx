import React, {useEffect, useState} from 'react';
import {useQuery} from '@tanstack/react-query';
import {Node} from './Graph';

export function NodeDetailsTooltip({
  nodeRef,
  node,
  graphType,
}: {
  nodeRef: React.RefObject<HTMLDivElement>;
  node: Node;
  graphType: string;
}) {
  const {
    data: nodeData,
    isLoading: isLoadingNodeData,
    error: errorNodeData,
  } = useQuery({
    queryKey: [`/api/${graphType}/node/${encodeURIComponent(node.id)}`],
  });
  const [position, setPosition] = useState({x: 0, y: 0});
  useEffect(() => {
    const parent = nodeRef.current?.parentElement?.parentElement;
    const parentRect = parent?.getBoundingClientRect();
    const rect = nodeRef.current?.getBoundingClientRect();

    if (!rect || !parentRect || !parent) {
      return;
    }

    setPosition({
      x: rect.left - parentRect.left + parent.scrollLeft,
      y: rect.bottom - parentRect.top,
    });
  }, [nodeRef]);

  return (
    <div
      style={{
        position: 'absolute',
        top: position.y,
        left: position.x,
        minWidth: '200px',
        minHeight: '200px',
        pointerEvents: 'none',
        zIndex: 1000,
        backgroundColor: 'white',
        border: '1px solid #ccc',
        borderRadius: '4px',
        padding: '10px',
      }}
    >
      {isLoadingNodeData ? (
        <div>Loading...</div>
      ) : errorNodeData ? (
        <div>Error: {errorNodeData.message}</div>
      ) : (
        <pre>{JSON.stringify(nodeData, null, 2)}</pre>
      )}
    </div>
  );
}
