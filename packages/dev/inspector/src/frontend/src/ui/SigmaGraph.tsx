// @ts-ignore
import Graphology from 'graphology';
import {useRef} from 'react';
// @ts-ignore
import forceAtlas2 from 'graphology-layout-forceatlas2';
// @ts-ignore
import FA2Layout from 'graphology-layout-forceatlas2/worker';
import {useEffect} from 'react';
// @ts-ignore
import Sigma from 'sigma';
import {Graph} from './Graph';

export function SigmaGraph({graph}: {graph: Graph<any>}) {
  const visualizationRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (visualizationRef.current) {
      const graphology = new Graphology();
      const nodes = new Set<string>();
      const edges = new Set<string>();
      for (let node of graph.nodes) {
        if (nodes.has(node.id)) {
          continue;
        }
        nodes.add(node.id);

        graphology.addNode(node.id, {
          label: node.displayName,
          // color: getRandomDarkerColor(node.displayName).family[2],
          x: Math.random() * 10000,
          y: Math.random() * 10000,
          size: 6,
        });
      }

      for (let node of graph.nodes) {
        for (let edge of node.edges) {
          if (nodes.has(node.id) && nodes.has(edge)) {
            if (edges.has(`${node.id} -> ${edge}`)) {
              continue;
            }

            edges.add(`${node.id} -> ${edge}`);

            graphology.addEdge(node.id, edge, {
              // color: 'red',
              // size: 0.1,
            });
          }
        }
      }

      const sensibleSettings = forceAtlas2.inferSettings(graphology);
      const fa2Layout = new FA2Layout(graphology, {
        settings: {
          ...sensibleSettings,
        },
      });
      fa2Layout.start();

      const renderer: any = new Sigma(graphology, visualizationRef.current, {
        allowInvalidContainer: true,
        defaultDrawNodeHover: () => {},
        labelRenderedSizeThreshold: 0,
      });

      renderer.on('enterNode', (e) => {
        console.log(e);
      });
      renderer.on('leaveNode', (e) => {
        console.log(e);
      });

      return () => {
        fa2Layout.stop();
        renderer.kill();
      };
    }
  }, [graph]);

  return (
    <div
      style={{height: '100%', width: '100%', flex: 1, position: 'relative'}}
      ref={visualizationRef}
    />
  );
}
