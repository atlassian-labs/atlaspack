// @flow strict-local

import path from 'path';
import assert from 'assert';
import {bundleGraphToRootedGraph} from '../../src/DominatorBundler/bundleGraphToRootedGraph';
import {findNodeEntryDependencies} from '../../src/DominatorBundler/findNodeEntryDependencies';
import {overlayFS, workerFarm} from '@atlaspack/test-utils';
import {asset, fixtureFromGraph} from '../fixtureFromGraph';
import {setupBundlerTest} from '../test-utils';
import nullthrows from 'nullthrows';

function mapToPaths(rootedGraph, map) {
  const result = new Map();
  for (const [key, value] of map.entries()) {
    const keyNode = nullthrows(rootedGraph.getNode(key));
    if (keyNode === 'root') throw new Error('Expected node to not be root');
    const valueNodes = [];
    for (const valueNode of value) {
      valueNodes.push(path.basename(valueNode.asset.filePath));
    }

    valueNodes.sort((a, b) => a.localeCompare(b));
    result.set(path.basename(keyNode.asset.filePath), valueNodes);
  }
  return result;
}

describe('findNodeEntryDependencies', () => {
  before(async function () {
    this.timeout(10000);
    // Warm up worker farm so that the first test doesn't account for this time.
    await workerFarm.callAllWorkers('ping', []);
  });

  it('returns the entry nodes for every node on the graph', async () => {
    const entryPath = path.join(__dirname, 'test/test.js');
    const entryDir = path.dirname(entryPath);
    await fixtureFromGraph(entryDir, overlayFS, [
      asset('test.js', ['dependency.js', {to: 'async.js', type: 'async'}]),
      asset('async.js', []),
      asset('dependency.js', []),
    ]);

    const {mutableBundleGraph} = await setupBundlerTest(entryPath);
    const rootedGraph = bundleGraphToRootedGraph(mutableBundleGraph).getGraph();

    const result = findNodeEntryDependencies(rootedGraph);

    assert.deepEqual(
      mapToPaths(rootedGraph, result.entryDependenciesByAsset),
      new Map([
        ['async.js', ['test.js']],
        ['dependency.js', ['test.js']],
        ['test.js', ['test.js']],
      ]),
    );

    assert.deepEqual(
      mapToPaths(rootedGraph, result.asyncDependenciesByAsset),
      new Map([['async.js', ['async.js']]]),
    );
  });

  it('works when there are multiple entry-points', async () => {
    const entryPath1 = path.join(__dirname, 'test/page1.js');
    const entryPath2 = path.join(__dirname, 'test/page2.js');
    const entryDir = path.dirname(entryPath1);
    await fixtureFromGraph(entryDir, overlayFS, [
      asset('page1.js', ['dependency.js', {to: 'async.js', type: 'async'}]),
      asset('page2.js', ['dependency.js', {to: 'async.js', type: 'async'}]),
      asset('async.js', []),
      asset('dependency.js', []),
    ]);

    const {mutableBundleGraph} = await setupBundlerTest([
      entryPath1,
      entryPath2,
    ]);
    const rootedGraph = bundleGraphToRootedGraph(mutableBundleGraph).getGraph();

    const result = findNodeEntryDependencies(rootedGraph);

    assert.deepEqual(
      mapToPaths(rootedGraph, result.entryDependenciesByAsset),
      new Map([
        ['async.js', ['page1.js', 'page2.js']],
        ['dependency.js', ['page1.js', 'page2.js']],
        ['page1.js', ['page1.js']],
        ['page2.js', ['page2.js']],
      ]),
    );

    assert.deepEqual(
      mapToPaths(rootedGraph, result.asyncDependenciesByAsset),
      new Map([['async.js', ['async.js']]]),
    );
  });

  it('works if there are multiple paths to dependency', async () => {
    const entryPath1 = path.join(__dirname, 'test/page1.js');
    const entryPath2 = path.join(__dirname, 'test/page2.js');
    const entryDir = path.dirname(entryPath1);
    await fixtureFromGraph(entryDir, overlayFS, [
      asset('page1.js', [{to: 'async.js', type: 'async'}]),
      asset('page2.js', ['dependency.js', {to: 'async.js', type: 'async'}]),
      asset('async.js', ['dependency.js']),
      asset('dependency.js', []),
    ]);

    const {mutableBundleGraph} = await setupBundlerTest([
      entryPath1,
      entryPath2,
    ]);
    const rootedGraph = bundleGraphToRootedGraph(mutableBundleGraph).getGraph();

    const result = findNodeEntryDependencies(rootedGraph);

    assert.deepEqual(
      mapToPaths(rootedGraph, result.entryDependenciesByAsset),
      new Map([
        ['async.js', ['page1.js', 'page2.js']],
        ['dependency.js', ['page1.js', 'page2.js']],
        ['page1.js', ['page1.js']],
        ['page2.js', ['page2.js']],
      ]),
    );

    assert.deepEqual(
      mapToPaths(rootedGraph, result.asyncDependenciesByAsset),
      new Map([
        ['async.js', ['async.js']],
        ['dependency.js', ['async.js']],
      ]),
    );
  });

  it('works with multiple async layers', async () => {
    const entryPath = path.join(__dirname, 'test/index.js');
    const entryDir = path.dirname(entryPath);
    await fixtureFromGraph(entryDir, overlayFS, [
      asset('index.js', [
        {to: 'page1.js', type: 'async'},
        {to: 'page2.js', type: 'async'},
      ]),
      asset('page1.js', [{to: 'async.js', type: 'async'}]),
      asset('page2.js', ['dependency.js', {to: 'async.js', type: 'async'}]),
      asset('async.js', ['dependency.js']),
      asset('dependency.js', []),
    ]);

    const {mutableBundleGraph} = await setupBundlerTest(entryPath);
    const rootedGraph = bundleGraphToRootedGraph(mutableBundleGraph).getGraph();

    const result = findNodeEntryDependencies(rootedGraph);

    assert.deepEqual(
      mapToPaths(rootedGraph, result.entryDependenciesByAsset),
      new Map([
        ['async.js', ['index.js']],
        ['dependency.js', ['index.js']],
        ['index.js', ['index.js']],
        ['page1.js', ['index.js']],
        ['page2.js', ['index.js']],
      ]),
    );

    assert.deepEqual(
      mapToPaths(rootedGraph, result.asyncDependenciesByAsset),
      new Map([
        ['page1.js', ['page1.js']],
        ['page2.js', ['page2.js']],
        ['async.js', ['async.js', 'page1.js', 'page2.js']],
        ['dependency.js', ['async.js', 'page1.js', 'page2.js']],
      ]),
    );
  });
});
