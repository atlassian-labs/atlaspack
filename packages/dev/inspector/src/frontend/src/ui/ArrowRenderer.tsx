import React, { useRef, useState, useEffect } from 'react';

export function ArrowRenderer({ connections, nodeRefs }) {
  const svgRef = useRef<SVGSVGElement>(null);
  const [paths, setPaths] = useState([]);
  const [size, setSize] = useState({ width: 0, height: 0 });

  useEffect(() => {
    const updatePaths = () => {
      if (!svgRef.current) return;

      const container = svgRef.current.closest('[style*="overflow: auto"]');
      if (!container) return;

      const containerRect = container.getBoundingClientRect();
      setSize({
        width: container.scrollWidth,
        height: container.scrollHeight,
      });
      const scrollLeft = container.scrollLeft;
      const scrollTop = container.scrollTop;

      const newPaths = connections
        .map(({ from, to }) => {
          const fromRef = nodeRefs.current.get(from);
          const toRef = nodeRefs.current.get(to);
          if (!fromRef?.current || !toRef?.current) return null;

          const fromRect = fromRef.current.getBoundingClientRect();
          const toRect = toRef.current.getBoundingClientRect();

          // Calculate start and end points relative to the container and scroll position
          const startX = fromRect.right - containerRect.left + scrollLeft;
          const startY = fromRect.top - containerRect.top + scrollTop + fromRect.height / 2;
          const endX = toRect.left - containerRect.left + scrollLeft;
          const endY = toRect.top - containerRect.top + scrollTop + toRect.height / 2;

          return {
            id: `${from}-${to}`,
            d: `M ${startX} ${startY} L ${endX - 5} ${endY}`,
          };
        })
        .filter(Boolean);

      setPaths(newPaths);
    };

    // Initial update
    updatePaths();

    // Update on window resize
    window.addEventListener('resize', updatePaths);
    return () => window.removeEventListener('resize', updatePaths);
  }, [connections, nodeRefs]);

  return (
    <svg
      ref={svgRef}
      style={{
        position: 'absolute',
        top: 0,
        left: 0,
        height: size.height,
        width: size.width,
        pointerEvents: 'none',
        zIndex: 1,
      }}
    >
      <defs>
        <marker
          id="arrowhead"
          markerWidth="6"
          markerHeight="4"
          refX="5"
          refY="2"
          orient="auto"
        >
          <polygon points="0 0, 6 2, 0 4" fill="#2196f3" />
        </marker>
      </defs>
      {paths.map(({ id, d }, i) => (
        <path
          key={i}
          d={d}
          stroke="#2196f3"
          strokeWidth={2}
          fill="none"
          markerEnd="url(#arrowhead)" />
      ))}
    </svg>
  );
}
