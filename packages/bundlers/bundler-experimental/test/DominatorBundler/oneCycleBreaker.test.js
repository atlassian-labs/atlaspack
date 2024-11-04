// @flow strict-local

import {ContentGraph} from '@atlaspack/graph';
import {
  convertToAcyclicGraph,
  findStronglyConnectedComponents,
} from '../../src/DominatorBundler/oneCycleBreaker';
import assert from 'assert';
import {dotTest, setupBundlerTest} from '../test-utils';
import {asset, fixtureFromGraph} from '../fixtureFromGraph';
import path from 'path';
import {overlayFS} from '@atlaspack/test-utils';
import {bundleGraphToRootedGraph} from '../../src/DominatorBundler/bundleGraphToRootedGraph';
import {rootedGraphToDot} from '../graphviz/GraphvizUtils';

describe.only('oneCycleBreaker', () => {
  describe('findStronglyConnectedComponents', () => {
    it('should find strongly connected components', () => {
      const graph = new ContentGraph();
      graph.addNodeByContentKey('root', 'root');
      graph.setRootNodeId(0);
      graph.addNodeByContentKey('a', 'a');
      graph.addNodeByContentKey('b', 'b');
      graph.addNodeByContentKey('c', 'c');

      graph.addEdge(0, 1);
      graph.addEdge(1, 2);
      graph.addEdge(2, 1);

      const result = findStronglyConnectedComponents(graph);

      assert.deepStrictEqual(result, [[2, 1], [0]]);
    });
  });

  describe('convertToAcyclicGraph', () => {
    it('should replace strongly connected components with a single node', () => {
      const graph = new ContentGraph();
      graph.addNodeByContentKey('root', 'root');
      graph.setRootNodeId(0);
      const a = graph.addNodeByContentKey('a', 'a');
      const b = graph.addNodeByContentKey('b', 'b');
      const c = graph.addNodeByContentKey('c', 'c');

      graph.addEdge(0, a);
      graph.addEdge(a, b);
      graph.addEdge(b, c);
      graph.addEdge(c, b);

      const result = convertToAcyclicGraph(graph);

      const nodes = [...result.nodes];
      nodes.sort();

      assert.deepStrictEqual(nodes, [
        {
          id: 'StronglyConnectedComponent:e9517a2f9d0a516b',
          type: 'StronglyConnectedComponent',
          nodeIds: [c, b],
          values: ['c', 'b'],
        },
        'a',
        'root',
      ]);
    });

    dotTest(
      __filename,
      'can replace strongly connected components with a single node on an example graph',
      async () => {
        const entryDir = path.join(__dirname, 'test');
        const entryPath1 = path.join(entryDir, 'page1.js');
        const entryPath2 = path.join(entryDir, 'page2.js');
        const inputDot = await fixtureFromGraph(entryDir, overlayFS, [
          asset('page1.js', ['react.js', 'lodash.js']),
          asset('page2.js', ['lodash.js', 'react.js']),
          asset('react.js', ['left-pad.js', 'string-concat.js', 'jsx.js']),
          asset('lodash.js', ['left-pad.js']),
          asset('left-pad.js', ['string-concat.js']),
          asset('jsx.js', []),
          asset('string-concat.js', ['string-chart-at.js']),
          asset('string-chart-at.js', []),
        ]);

        const {mutableBundleGraph} = await setupBundlerTest([
          entryPath1,
          entryPath2,
        ]);
        const rootedGraph = bundleGraphToRootedGraph(mutableBundleGraph);
        const stronglyConnectedComponents =
          findStronglyConnectedComponents(rootedGraph);

        assert.deepStrictEqual(
          stronglyConnectedComponents
            .filter((cs) => cs.length > 1)
            .map((cs) =>
              cs.map((c) => {
                const node = rootedGraph.getNode(c);
                if (node == null) {
                  return null;
                } else if (node === 'root') {
                  return 'root';
                } else {
                  return path.basename(node.filePath);
                }
              }),
            ),
          [['lodash.js', 'page2.js', 'root']],
        );

        const result = convertToAcyclicGraph(rootedGraph);

        return [
          {label: 'input', dot: inputDot},
          {label: 'no-cycles', dot: rootedGraphToDot(entryDir, result)},
        ];
      },
    );
  });
});
