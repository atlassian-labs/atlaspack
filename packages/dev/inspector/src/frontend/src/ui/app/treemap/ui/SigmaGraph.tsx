import Graphology from 'graphology';
import {useRef} from 'react';
import forceAtlas2 from 'graphology-layout-forceatlas2';
import FA2Layout from 'graphology-layout-forceatlas2/worker';
import {useEffect} from 'react';
import Sigma from 'sigma';

import {Graph} from '../../../types/Graph';
import styles from './SigmaGraph.module.css';

/**
 * Renders `Graph` visualisation using Sigma.js.
 */
export function SigmaGraph<T>({graph}: {graph: Graph<T>}) {
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

            graphology.addEdge(node.id, edge, {});
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

      const renderer = new Sigma(graphology, visualizationRef.current, {
        allowInvalidContainer: true,
        defaultDrawNodeHover: () => {},
        labelRenderedSizeThreshold: 0,
      });

      // TODO: Listen to enter/leave and highlight the rows
      // renderer.on('enterNode', (e) => {
      //   console.log(e);
      // });
      // renderer.on('leaveNode', (e) => {
      //   console.log(e);
      // });

      return () => {
        fa2Layout.stop();
        renderer.kill();
      };
    }
  }, [graph]);

  return <div className={styles.expander} ref={visualizationRef} />;
}
