// @flow strict-local

import path from 'path';
import expect from 'expect';
import {overlayFS, workerFarm} from '@atlaspack/test-utils';
import {dotTest, setupBundlerTest, testMakePackageKey} from '../test-utils';
import {
  createPackages,
  getAssetNodeByKey,
  getChunkEntryPoints,
} from '../../src/DominatorBundler/createPackages';
import assert from 'assert';
import {asset, fixtureFromGraph} from '../fixtureFromGraph';
import {
  rootedGraphToDot,
  mergedDominatorsToDot,
  cleanPath,
} from '../graphviz/GraphvizUtils';
import {findAssetDominators} from '../../src/DominatorBundler/findAssetDominators';
import {bundleGraphToRootedGraph} from '../../src/DominatorBundler/bundleGraphToRootedGraph';

async function testGetPackages(entryPath: string, entryDir: string) {
  const {mutableBundleGraph} = await setupBundlerTest(entryPath);
  const {dominators} = findAssetDominators(mutableBundleGraph);
  const packages = createPackages(
    mutableBundleGraph,
    dominators,
    (parentChunks) => testMakePackageKey(entryDir, dominators, parentChunks),
  );
  return {packages};
}

describe('DominatorBundler/createPackages', () => {
  before(async function () {
    this.timeout(10000);
    // Warm up worker farm so that the first test doesn't account for this time.
    await workerFarm.callAllWorkers('ping', []);
  });

  it('does nothing if there is just one node in the graph', async () => {
    const entryDir = path.join(__dirname, 'test');
    const entryPath = path.join(entryDir, 'a.js');
    await fixtureFromGraph(entryDir, overlayFS, [asset('a.js', [])]);
    const {packages} = await testGetPackages(entryPath, entryDir);
    const dot = mergedDominatorsToDot(entryDir, packages);
    expect(dot).toEqual(
      `
digraph merged {
  labelloc="t";
  label="Merged";
  layout="dot";

  "a.js";
  "root";

  "root" -> "a.js";
}
      `.trim(),
    );
  });

  it('does nothing if there is just one dependency', async () => {
    const entryDir = path.join(__dirname, 'test');
    const entryPath = path.join(entryDir, 'a.js');
    await fixtureFromGraph(entryDir, overlayFS, [
      asset('a.js', ['b.js']),
      asset('b.js', []),
    ]);
    const {packages} = await testGetPackages(entryPath, entryDir);
    const dot = mergedDominatorsToDot(entryDir, packages);
    expect(dot).toEqual(
      `
digraph merged {
  labelloc="t";
  label="Merged";
  layout="dot";

  "a.js";
  "b.js";
  "root";

  "a.js" -> "b.js";
  "root" -> "a.js";
}
      `.trim(),
    );
  });

  it('does nothing if all nodes are reachable from the root', async () => {
    const entryDir = path.join(__dirname, 'test');
    const entryPath = path.join(entryDir, 'a.js');
    await fixtureFromGraph(entryDir, overlayFS, [
      asset('a.js', ['b.js', {to: 'async.js', type: 'async'}]),
      asset('b.js', []),
      asset('async.js', []),
    ]);
    const {packages} = await testGetPackages(entryPath, entryDir);
    const dot = mergedDominatorsToDot(entryDir, packages);
    expect(dot).toEqual(
      `
digraph merged {
  labelloc="t";
  label="Merged";
  layout="dot";

  "a.js";
  "async.js";
  "b.js";
  "root";

  "a.js" -> "b.js";
  "root" -> "a.js";
  "root" -> "async.js";
}
      `.trim(),
    );
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
      const {dominators} = findAssetDominators(mutableBundleGraph);
      const rootedGraph = bundleGraphToRootedGraph(mutableBundleGraph);

      const chunkEntryPoints = getChunkEntryPoints(rootedGraph, dominators);

      const filePathEntryPoints = {};
      for (const [chunkId, {entryPointChunkIds}] of chunkEntryPoints) {
        const chunk = getAssetNodeByKey(dominators, chunkId);
        const entryPoints = Array.from(entryPointChunkIds).map((id) =>
          cleanPath(entryDir, getAssetNodeByKey(dominators, id).filePath),
        );

        entryPoints.sort((a, b) => a.localeCompare(b));

        if (chunk.filePath.includes('esmodule-helpers.js')) {
          continue;
        }
        filePathEntryPoints[cleanPath(entryDir, chunk.filePath)] = entryPoints;
      }

      assert.deepEqual(filePathEntryPoints, {
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

        const rootedGraph = bundleGraphToRootedGraph(mutableBundleGraph);
        const {dominators} = findAssetDominators(mutableBundleGraph);
        const outputDot = rootedGraphToDot(entryDir, dominators);

        const mergedDominators = createPackages(
          mutableBundleGraph,
          dominators,
          (parentChunks) =>
            testMakePackageKey(entryDir, dominators, parentChunks),
        );
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

  "jsx.js";
  "left-pad.js";
  "lodash.js";
  "package:page1.js,page2.js";
  "page1.js";
  "page2.js";
  "react.js";
  "root";
  "string-chart-at.js";
  "string-concat.js";

  "package:page1.js,page2.js" -> "left-pad.js";
  "package:page1.js,page2.js" -> "lodash.js";
  "package:page1.js,page2.js" -> "react.js";
  "package:page1.js,page2.js" -> "string-concat.js";
  "react.js" -> "jsx.js";
  "root" -> "package:page1.js,page2.js";
  "root" -> "page1.js";
  "root" -> "page2.js";
  "string-concat.js" -> "string-chart-at.js";
}
                  `.trim(),
        );

        return [
          {label: 'input', dot: inputDot},
          {label: 'root', dot: rootedGraphToDot(entryDir, rootedGraph)},
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
