// @ts-ignore
import Graphology from 'graphology';
import { useRef } from 'react';
// @ts-ignore
import forceAtlas2 from 'graphology-layout-forceatlas2';
// @ts-ignore
import FA2Layout from 'graphology-layout-forceatlas2/worker';
import { useEffect } from 'react';
// @ts-ignore
import Sigma from 'sigma';
import { useSearchParams } from 'react-router';
import { useQuery } from '@tanstack/react-query';
import qs from 'qs';
import { Graph } from './Graph';
import { getRandomDarkerColor } from './getRandomDarkerColor';

function setup(container: HTMLDivElement, graph: Graphology) {
  const sensibleSettings = forceAtlas2.inferSettings(graph);
  const fa2Layout = new FA2Layout(graph, {
    settings: sensibleSettings,
  });
  fa2Layout.start();
  setTimeout(() => {
    fa2Layout.stop();
  }, 8000);

  const renderer: any = new Sigma(graph, container);

  return () => {
    renderer.kill();
  };
}

export function SigmaPage() {
  const [searchParams] = useSearchParams();
  const rootNodeId = searchParams.get('rootNodeId');
  const visualizationRef = useRef<HTMLDivElement>(null);
  const {
    data: bundleGraph,
    isLoading: isLoadingBundleGraph,
    error: errorBundleGraph,
  } = useQuery<Graph>({
    queryKey: [`/api/bundle-graph?${qs.stringify({rootNodeId})}`],
  });

  useEffect(() => {
    if (visualizationRef.current && bundleGraph) {
      const graph = new Graphology();
      const nodes = new Set<string>();
      for (let node of bundleGraph.nodes) {
        nodes.add(node.id);
        graph.addNode(node.id, {
          label: node.displayName,
          color: getRandomDarkerColor(node.displayName).family[2],
          x: Math.random() * 10000,
          y: Math.random() * 10000,
          size: 5,
        });
      }
      for (let node of bundleGraph.nodes) {
        for (let edge of node.edges) {
          if (nodes.has(edge)) {
            graph.addEdge(node.id, edge, {
              color: 'red',
              size: 1,
            });
            console.log(`Added edge ${node.id} -> ${edge}`);
          } else {
            // console.warn(`Edge ${edge} not found from ${node.id}`);
          }
        }
      }

      return setup(visualizationRef.current, graph);
    }
  }, [bundleGraph]);

  if (isLoadingBundleGraph) {
    return <div>Loading...</div>;
  }

  if (errorBundleGraph) {
    return <div>Error: {errorBundleGraph.message}</div>;
  }

  if (!bundleGraph) {
    throw new Error('No bundle graph');
  }

  return (
    <div
      style={{height: '100%', width: '100%', flex: 1}}
      ref={visualizationRef}
    />
  );
}
