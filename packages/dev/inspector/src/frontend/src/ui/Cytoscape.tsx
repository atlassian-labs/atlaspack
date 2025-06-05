import cytoscape from 'cytoscape';
import {useRef, useEffect} from 'react';
import {useSearchParams} from 'react-router';
import {useQuery} from '@tanstack/react-query';
import qs from 'qs';
import {Graph} from './Graph';
// @ts-ignore
import dagre from 'cytoscape-dagre';

export function CytoscapePage() {
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
      cytoscape.use(dagre);

      const cy = cytoscape({
        container: visualizationRef.current,
        layout: {
          name: 'dagre',
        },
        style: [
          // the stylesheet for the graph
          {
            selector: 'node',
            style: {
              'background-color': '#666',
              label: 'data(id)',
            },
          },

          {
            selector: 'edge',
            style: {
              width: 3,
              'line-color': '#ccc',
              'target-arrow-color': '#ccc',
              'target-arrow-shape': 'triangle',
              'curve-style': 'bezier',
            },
          },
        ],
      });

      cy.batch(() => {
        const nodes = new Set<string>();

        for (let node of bundleGraph.nodes) {
          nodes.add(node.id);
          cy.add({
            group: 'nodes',
            data: {
              id: node.id,
              label: node.displayName ?? node.id,
              // color: getRandomDarkerColor(node.displayName).family[2],
            },
            position: {
              x: Math.random() * 10000 - 5000,
              y: Math.random() * 10000 - 5000,
            },
          });

          if (nodes.size > 1000) {
            break;
          }
        }

        for (let node of bundleGraph.nodes) {
          for (let edge of node.edges) {
            if (nodes.has(node.id) && nodes.has(edge)) {
              cy.add({
                group: 'edges',
                data: {
                  id: `${node.id}-${edge}`,
                  source: node.id,
                  target: edge,
                  // color: 'red',
                  // size: 1,
                },
              });
            } else {
              // console.warn(`Edge ${edge} not found from ${node.id}`);
            }
          }
        }
      });

      cy.layout({
        name: 'dagre',
        // @ts-ignore
        align: 'DR',
      }).run();

      const rootNode = bundleGraph.nodes[0];
      cy.fit(cy.$(`#${rootNode.id}`));

      return () => {
        cy.destroy();
      };
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
