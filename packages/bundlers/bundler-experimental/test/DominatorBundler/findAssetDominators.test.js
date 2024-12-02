// @flow strict-local

import path from 'path';
import {overlayFS, workerFarm} from '@atlaspack/test-utils';
import assert from 'assert';
import {dotTest, setupBundlerTest} from '../test-utils';
import {asset, fixtureFromGraph} from '../fixtureFromGraph';
import {rootedGraphToDot} from '../graphviz/GraphvizUtils';
import {
  buildDominatorTree,
  findAssetDominators,
  simpleFastDominance,
} from '../../src/DominatorBundler/findAssetDominators';
import {EdgeContentGraph} from '../../src/DominatorBundler/EdgeContentGraph';

describe('findAssetDominators', () => {
  before(async function () {
    this.timeout(10000);
    // Warm up worker farm so that the first test doesn't account for this time.
    await workerFarm.callAllWorkers('ping', []);
  });

  const baseGraph = () => {
    const inputGraph = new EdgeContentGraph();
    const root = inputGraph.addNodeByContentKey('root', 'root');
    inputGraph.setRootNodeId(root);
    return {inputGraph, root};
  };

  describe('findAssetDominators', () => {
    it('works on an empty graph', () => {
      // digraph g {
      //   root
      // }
      const {inputGraph, root} = baseGraph();
      const dominators = simpleFastDominance(inputGraph);
      const result = buildDominatorTree(inputGraph, dominators);
      assert.deepEqual(result.nodes, [result.getNodeByContentKey('root')]);
    });

    it('works on a single node graph', () => {
      // digraph g {
      //   root -> a
      // }
      const {inputGraph, root} = baseGraph();
      const a = inputGraph.addNodeByContentKey('a', 'a');

      inputGraph.addWeightedEdge(root, a, 1, 'weight');

      const dominators = simpleFastDominance(inputGraph);

      const result = buildDominatorTree(inputGraph, dominators);
      assert.deepEqual(result.nodes, [
        result.getNodeByContentKey('root'),
        result.getNodeByContentKey('a'),
      ]);
      assert.deepEqual(Array.from(result.getAllEdges()), [
        {
          from: root,
          to: a,
          type: 1,
        },
      ]);
      assert.deepEqual(
        result.getEdgeWeight(
          result.getNodeIdByContentKey('root'),
          result.getNodeIdByContentKey('a'),
        ),
        'weight',
      );
    });

    it('it works on a linear graph', () => {
      // digraph g {
      //   root -> a -> b -> c -> d
      // }
      const {inputGraph, root} = baseGraph();
      inputGraph.setRootNodeId(root);

      const a = inputGraph.addNodeByContentKey('a', 'a');
      const b = inputGraph.addNodeByContentKey('b', 'b');
      const c = inputGraph.addNodeByContentKey('c', 'c');
      const d = inputGraph.addNodeByContentKey('d', 'd');

      inputGraph.addEdge(root, a);
      inputGraph.addEdge(a, b);
      inputGraph.addEdge(b, c);
      inputGraph.addEdge(c, d);

      const dominators = simpleFastDominance(inputGraph);
      assert.equal(dominators[root], root);
      assert.equal(dominators[a], root);
      assert.equal(dominators[b], a);
      assert.equal(dominators[c], b);
      assert.equal(dominators[d], c);
    });

    it('it works on a tree graph', () => {
      // digraph g {
      //   root -> a;
      //   root -> b;
      //   a -> c;
      //   a -> d;
      //   b -> e;
      // }
      const {inputGraph, root} = baseGraph();

      const a = inputGraph.addNodeByContentKey('a', 'a');
      const b = inputGraph.addNodeByContentKey('b', 'b');
      const c = inputGraph.addNodeByContentKey('c', 'c');
      const d = inputGraph.addNodeByContentKey('d', 'd');
      const e = inputGraph.addNodeByContentKey('e', 'e');

      inputGraph.addEdge(root, a);
      inputGraph.addEdge(root, b);
      inputGraph.addEdge(a, c);
      inputGraph.addEdge(a, d);
      inputGraph.addEdge(b, e);

      const dominators = simpleFastDominance(inputGraph);
      assert.equal(dominators[root], root);
      assert.equal(dominators[a], root);
      assert.equal(dominators[b], root);
      assert.equal(dominators[c], a);
      assert.equal(dominators[d], a);
      assert.equal(dominators[e], b);
    });

    it('it works on a graph with multiple paths to nodes', () => {
      // digraph g {
      //   root -> a;
      //   root -> b;
      //   a -> c;
      //   a -> d;
      //   b -> d;
      //   d -> c;
      // }
      const {inputGraph, root} = baseGraph();

      const a = inputGraph.addNodeByContentKey('a', 'a');
      const b = inputGraph.addNodeByContentKey('b', 'b');
      const c = inputGraph.addNodeByContentKey('c', 'c');
      const d = inputGraph.addNodeByContentKey('d', 'd');

      inputGraph.addEdge(root, a);
      inputGraph.addEdge(root, b);
      inputGraph.addEdge(a, c);
      inputGraph.addEdge(a, d);
      inputGraph.addEdge(b, d);
      inputGraph.addEdge(d, c);

      const dominators = simpleFastDominance(inputGraph);
      assert.equal(dominators[root], root);
      assert.equal(dominators[a], root);
      assert.equal(dominators[b], root);
      assert.equal(dominators[c], root);
      assert.equal(dominators[d], root);
    });

    describe('fixture tests', () => {
      dotTest(
        __filename,
        'can find dominators for a simple graph',
        async () => {
          const entryPath = path.join(__dirname, 'test/test.js');
          const entryDir = path.dirname(entryPath);
          const inputDot = await fixtureFromGraph(entryDir, overlayFS, [
            asset('test.js', [
              'dependency.js',
              {to: 'async.js', type: 'async'},
            ]),
            asset('async.js', []),
            asset('dependency.js', []),
          ]);

          const {mutableBundleGraph} = await setupBundlerTest(entryPath);
          const {dominators} = findAssetDominators(mutableBundleGraph);

          const outputDot = rootedGraphToDot(entryDir, dominators);
          assert.equal(
            outputDot,
            `
digraph dominators {
  labelloc="t";
  label="Dominators";

  "root";
  "root" -> "async.js";
  "root" -> "test.js";
  "async.js";
  "dependency.js";
  "test.js";

  "test.js" -> "dependency.js";
}
      `.trim(),
          );

          return [
            {label: 'input', dot: inputDot},
            {label: 'output', dot: outputDot},
          ];
        },
      );

      dotTest(
        __filename,
        'can find dominators for a slightly more complex graph',
        async () => {
          const entryPath = path.join(__dirname, 'test/page.js');
          const entryDir = path.dirname(entryPath);
          const inputDot = await fixtureFromGraph(entryDir, overlayFS, [
            asset('page.js', ['react.js', 'lodash.js']),
            asset('react.js', ['left-pad.js', 'string-concat.js', 'jsx.js']),
            asset('lodash.js', ['left-pad.js']),
            asset('left-pad.js', ['string-concat.js']),
            asset('jsx.js', []),
            asset('string-concat.js', ['string-chart-at.js']),
            asset('string-chart-at.js', []),
          ]);

          const {mutableBundleGraph} = await setupBundlerTest(entryPath);
          const {dominators} = findAssetDominators(mutableBundleGraph);

          const outputDot = rootedGraphToDot(entryDir, dominators);
          assert.equal(
            outputDot,
            `
digraph dominators {
  labelloc="t";
  label="Dominators";

  "root";
  "root" -> "page.js";
  "jsx.js";
  "left-pad.js";
  "lodash.js";
  "page.js";
  "react.js";
  "string-chart-at.js";
  "string-concat.js";

  "page.js" -> "left-pad.js";
  "page.js" -> "lodash.js";
  "page.js" -> "react.js";
  "page.js" -> "string-concat.js";
  "react.js" -> "jsx.js";
  "string-concat.js" -> "string-chart-at.js";
}
            `.trim(),
          );

          return [
            {label: 'input', dot: inputDot},
            {label: 'output', dot: outputDot},
          ];
        },
      );

      dotTest(
        __filename,
        'works when there are multiple entry-points',
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
          const {dominators} = findAssetDominators(mutableBundleGraph);

          const outputDot = rootedGraphToDot(entryDir, dominators);
          assert.equal(
            outputDot,
            `
digraph dominators {
  labelloc="t";
  label="Dominators";

  "root";
  "root" -> "left-pad.js";
  "root" -> "lodash.js";
  "root" -> "page1.js";
  "root" -> "page2.js";
  "root" -> "react.js";
  "root" -> "string-concat.js";
  "jsx.js";
  "left-pad.js";
  "lodash.js";
  "page1.js";
  "page2.js";
  "react.js";
  "string-chart-at.js";
  "string-concat.js";

  "react.js" -> "jsx.js";
  "string-concat.js" -> "string-chart-at.js";
}
            `.trim(),
          );

          return [
            {label: 'input', dot: inputDot},
            {label: 'output', dot: outputDot},
          ];
        },
      );
    });
  });
});
