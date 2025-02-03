// @flow strict-local

import {
  convertToAcyclicGraph,
  findStronglyConnectedComponents,
} from '../../src/DominatorBundler/cycleBreaker';
import assert from 'assert';
import {dotTest, setupBundlerTest} from '../test-utils';
import {asset, fixtureFromGraph} from '../fixtureFromGraph';
import path from 'path';
import {overlayFS, workerFarm} from '@atlaspack/test-utils';
import {bundleGraphToRootedGraph} from '../../src/DominatorBundler/bundleGraphToRootedGraph';
import {rootedGraphToDot} from '../graphviz/GraphvizUtils';
import {EdgeContentGraph} from '../../src/DominatorBundler/EdgeContentGraph';

describe('cycleBreaker', () => {
  before(async function () {
    this.timeout(10000);
    // Warm up worker farm so that the first test doesn't account for this time.
    await workerFarm.callAllWorkers('ping', []);
  });

  describe('findStronglyConnectedComponents', () => {
    it('should find strongly connected components', () => {
      const graph = new EdgeContentGraph();
      const root = graph.addNodeByContentKey('root', 'root');
      graph.setRootNodeId(0);
      const a = graph.addNodeByContentKey('a', 'a');
      const b = graph.addNodeByContentKey('b', 'b');
      const c = graph.addNodeByContentKey('c', 'c');

      graph.addEdge(root, a);
      graph.addEdge(a, b);
      graph.addEdge(b, a);

      const result = findStronglyConnectedComponents(graph);

      assert.deepStrictEqual(result, [[b, a], [root], [c]]);
    });
  });

  describe('convertToAcyclicGraph', () => {
    it('should replace strongly connected components with a single node', () => {
      const graph = new EdgeContentGraph();
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

      const nodes = [...result.nodes].filter((n) => n != null);
      nodes.sort();

      assert.deepStrictEqual(nodes, [
        {
          id: 'StronglyConnectedComponent:e9517a2f9d0a516b',
          type: 'StronglyConnectedComponent',
          nodeIds: [c, b],
          values: ['c', 'b'],
        },
        'a',
        'b',
        'c',
        'root',
      ]);
    });

    it('works on a graph with two starting roots', () => {
      const graph = new EdgeContentGraph();
      graph.setRootNodeId(0);
      const root = graph.addNodeByContentKey('root', 'root');
      const a = graph.addNodeByContentKey('a', 'a');
      const b = graph.addNodeByContentKey('b', 'b');
      const c = graph.addNodeByContentKey('c', 'c');
      const d = graph.addNodeByContentKey('d', 'd');
      const e = graph.addNodeByContentKey('e', 'e');

      graph.addWeightedEdge(root, a, 1, 'a-weight');
      graph.addWeightedEdge(root, b, 1, 'b-weight');
      graph.addEdge(a, c);
      graph.addEdge(b, c);
      graph.addEdge(c, d);
      graph.addEdge(d, e);
      graph.addEdge(e, c);
      /*

        root -> a -> c -> d -> e -----+
            \-- b--/ |----------------+
       */
      // $FlowFixMe
      const result = convertToAcyclicGraph(graph);

      const nodes = [...result.nodes].filter((n) => n != null);
      nodes.sort();

      assert.equal(result.getEdgeWeight(root, a), 'a-weight');
      assert.equal(result.getEdgeWeight(root, b), 'b-weight');

      assert.deepStrictEqual(nodes, [
        {
          // $FlowFixMe
          id: nodes[0].id,
          type: 'StronglyConnectedComponent',
          nodeIds: [e, d, c],
          values: ['e', 'd', 'c'],
        },
        'a',
        'b',
        'c',
        'd',
        'e',
        'root',
      ]);
    });

    describe('fixture tests', () => {
      dotTest(__filename, 'cycles', async () => {
        const entryDir = path.join(__dirname, 'test');
        const entryPath1 = path.join(entryDir, 'a.js');
        const inputDot = await fixtureFromGraph(entryDir, overlayFS, [
          asset('a.js', ['b.js']),
          asset('b.js', ['c.js']),
          asset('c.js', ['d.js']),
          asset('d.js', ['b.js', 'e.js']),
          asset('e.js', []),
        ]);

        const {mutableBundleGraph} = await setupBundlerTest([entryPath1]);
        const rootedGraph = bundleGraphToRootedGraph(mutableBundleGraph);
        const result = convertToAcyclicGraph(rootedGraph);

        return [
          {label: 'input', dot: inputDot},
          {label: 'no-cycles', dot: rootedGraphToDot(entryDir, result)},
        ];
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
                    return path.basename(node.asset.filePath);
                  }
                }),
              ),
            [],
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
});
