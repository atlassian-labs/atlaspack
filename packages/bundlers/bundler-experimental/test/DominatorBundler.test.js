// @flow strict-local

import * as path from 'path';
import {overlayFS, workerFarm} from '@atlaspack/test-utils';
import {dotTest, setupBundlerTest} from './test-utils';
import {
  createPackages,
  getAssetNodeByKey,
  getChunkEntryPoints,
} from '../src/DominatorBundler';
import assert from 'assert';
import {asset, fixtureFromGraph} from './fixtureFromGraph';
import {
  rootedGraphToDot,
  mergedDominatorsToDot,
  cleanPath,
} from './graphviz/GraphvizUtils';
import {findAssetDominators} from '../src/DominatorBundler/findAssetDominators';
import {bundleGraphToRootedGraph} from '../src/DominatorBundler/bundleGraphToRootedGraph';

describe('DominatorBundler', () => {
  before(async function () {
    this.timeout(10000);
    // Warm up worker farm so that the first test doesn't account for this time.
    await workerFarm.callAllWorkers('ping', []);
  });

  describe('getChunkEntryPoints', () => {
    it('returns the entry points for a chunk', async () => {
      const entryDir = path.join(__dirname, 'test');
      const entryPath1 = path.join(entryDir, 'page1.js');
      const entryPath2 = path.join(entryDir, 'page2.js');
      await fixtureFromGraph(entryDir, overlayFS, [
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
      const dominators = findAssetDominators(mutableBundleGraph);
      const rootedGraph = bundleGraphToRootedGraph(mutableBundleGraph);

      const chunkEntryPoints = getChunkEntryPoints(rootedGraph, dominators);

      const filePathEntryPoints = {};
      for (const [chunkId, entryPointIds] of chunkEntryPoints) {
        const chunk = getAssetNodeByKey(dominators, chunkId);
        const entryPoints = Array.from(entryPointIds).map((id) =>
          cleanPath(entryDir, getAssetNodeByKey(dominators, id).filePath),
        );

        entryPoints.sort((a, b) => a.localeCompare(b));

        filePathEntryPoints[cleanPath(entryDir, chunk.filePath)] = entryPoints;
      }

      assert.deepEqual(filePathEntryPoints, {
        'esmodule_helpers.js': ['page1.js', 'page2.js'],
        'left-pad.js': ['page1.js', 'page2.js'],
        'lodash.js': ['page1.js', 'page2.js'],
        'react.js': ['page1.js', 'page2.js'],
        'string-concat.js': ['page1.js', 'page2.js'],
      });
    });
  });

  describe('createPackages - all together now', () => {
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
        const dominators = findAssetDominators(mutableBundleGraph);

        const outputDot = rootedGraphToDot(entryDir, dominators);

        const mergedDominators = createPackages(mutableBundleGraph, dominators);
        const mergedDominatorsDot = mergedDominatorsToDot(
          entryDir,
          mergedDominators,
        );

        assert.equal(
          mergedDominatorsDot,
          `
digraph merged {
  labelloc="t";
  label="Merged";
  layout="dot";

  "esmodule_helpers.js";
  "jsx.js";
  "left-pad.js";
  "lodash.js";
  "package_6b334e67b36bf482,a5fb992cd851a2a3";
  "page1.js";
  "page2.js";
  "react.js";
  "root";
  "string-chart-at.js";
  "string-concat.js";

  "package_6b334e67b36bf482,a5fb992cd851a2a3" -> "esmodule_helpers.js";
  "package_6b334e67b36bf482,a5fb992cd851a2a3" -> "left-pad.js";
  "package_6b334e67b36bf482,a5fb992cd851a2a3" -> "lodash.js";
  "package_6b334e67b36bf482,a5fb992cd851a2a3" -> "react.js";
  "package_6b334e67b36bf482,a5fb992cd851a2a3" -> "string-concat.js";
  "react.js" -> "jsx.js";
  "root" -> "package_6b334e67b36bf482,a5fb992cd851a2a3";
  "root" -> "page1.js";
  "root" -> "page2.js";
  "string-concat.js" -> "string-chart-at.js";
}
                  `.trim(),
        );

        return [
          {label: 'input', dot: inputDot},
          {label: 'output', dot: outputDot},
          {
            label: 'merged',
            dot: mergedDominatorsDot,
          },
        ];
      },
    );
  });
});
