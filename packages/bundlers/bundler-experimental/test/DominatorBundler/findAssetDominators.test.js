// @flow strict-local

import * as path from 'path';
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

  describe('findAssetDominators', () => {
    it('works on an empty graph', () => {
      const inputGraph = new EdgeContentGraph();
      const root = inputGraph.addNodeByContentKey('root', 'root');
      inputGraph.setRootNodeId(root);
      const dominators = simpleFastDominance(inputGraph);
      const result = buildDominatorTree(inputGraph, dominators);
      assert.deepEqual(result.nodes, [result.getNodeByContentKey('root')]);
    });

    it('works on a single node graph', () => {
      const inputGraph = new EdgeContentGraph();
      const root = inputGraph.addNodeByContentKey('root', 'root');
      inputGraph.setRootNodeId(root);
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

    dotTest(__filename, 'can find dominators for a simple graph', async () => {
      const entryPath = path.join(__dirname, 'test/test.js');
      const entryDir = path.dirname(entryPath);
      const inputDot = await fixtureFromGraph(entryDir, overlayFS, [
        asset('test.js', ['dependency.js', {to: 'async.js', type: 'async'}]),
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
    });

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
