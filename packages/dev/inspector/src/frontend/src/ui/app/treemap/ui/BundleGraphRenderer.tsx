import Graphology from 'graphology';
import {useRef} from 'react';
import forceAtlas2 from 'graphology-layout-forceatlas2';
import FA2Layout from 'graphology-layout-forceatlas2/worker';
import {useEffect} from 'react';
import Sigma from 'sigma';
import {useSearchParams} from 'react-router';
import {useQuery} from '@tanstack/react-query';
import qs from 'qs';
import {Graph} from '../../../types/Graph';
import Spinner from '@atlaskit/spinner';

function setup(container: HTMLDivElement, graph: Graphology) {
  const sensibleSettings = forceAtlas2.inferSettings(graph);
  const fa2Layout = new FA2Layout(graph, {
    settings: sensibleSettings,
  });
  fa2Layout.start();

  const renderer: any = new Sigma(graph, container);

  return () => {
    renderer.kill();
  };
}

type BundleGraph = Graph<{size: number}>;

export function BundleGraphRenderer() {
  const [searchParams] = useSearchParams();
  const rootNodeId = searchParams.get('rootNodeId');
  const visualizationRef = useRef<HTMLDivElement>(null);
  const {
    data: bundleGraph,
    isLoading: isLoadingBundleGraph,
    error: errorBundleGraph,
  } = useQuery<BundleGraph>({
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
          // color: getRandomDarkerColor(node.displayName).family[2],
          x: Math.random() * 10000,
          y: Math.random() * 10000,
          size:
            node.id === '@@root'
              ? 4
              : node.extra?.size
              ? node.extra.size / 500000
              : 2,
        });
      }
      for (let node of bundleGraph.nodes) {
        for (let edge of node.edges) {
          if (nodes.has(node.id) && nodes.has(edge)) {
            graph.addEdge(node.id, edge, {
              size: 0.1,
            });
          }
        }
      }

      return setup(visualizationRef.current, graph);
    }
  }, [bundleGraph]);

  if (isLoadingBundleGraph) {
    return (
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          height: '100%',
          width: '100%',
        }}
      >
        <Spinner size="large" />
      </div>
    );
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
