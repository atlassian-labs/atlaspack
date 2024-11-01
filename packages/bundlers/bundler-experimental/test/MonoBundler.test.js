// @flow strict-local

import {
  overlayFS,
  fsFixture,
  expectBundles,
  workerFarm,
} from '@atlaspack/test-utils';
import {monoBundler} from '../src/MonoBundler';
import * as path from 'path';
import {dotTest, setupBundlerTest} from './test-utils';
import {dotFromBundleGraph} from './graphviz/GraphvizUtils';
import {fixtureFromGraph, asset} from './fixtureFromGraph';

describe('MonoBundler', () => {
  before(async function () {
    this.timeout(10000);
    // Warm up worker farm so that the first test doesn't account for this time.
    await workerFarm.callAllWorkers('ping', []);
  });

  dotTest(__filename, 'can bundle a single file', async () => {
    const entryPath = path.join(__dirname, 'test/test.js');
    const entryDir = path.dirname(entryPath);
    const inputDot = await fixtureFromGraph(
      path.dirname(entryPath),
      overlayFS,
      [asset('test.js', [])],
    );

    const {mutableBundleGraph, entries} = await setupBundlerTest(entryPath);

    const {bundleGraph: outputBundleGraph} = monoBundler({
      inputGraph: mutableBundleGraph,
      outputGraph: mutableBundleGraph,
      entries,
    });

    expectBundles(entryDir, outputBundleGraph, [
      {
        assets: ['test.js'],
      },
    ]);

    const outputBundleGraphDot = dotFromBundleGraph(
      entryDir,
      outputBundleGraph,
    );

    return [
      {label: 'assets', dot: inputDot},
      {label: 'output', dot: outputBundleGraphDot},
    ];
  });

  it('can bundle multiple files with sync dependencies', async () => {
    const entryPath = path.join(__dirname, 'test/test.js');
    await fsFixture(overlayFS, __dirname)`
      test
        dependency.js:
          export function something(name) { return 'hey ' + name; };
        test.js:
          import {something} from './dependency';
          export function main() { return something('atlaspack'); };
    `;

    const {mutableBundleGraph, entries} = await setupBundlerTest(entryPath);

    const {bundleGraph: outputBundleGraph} = monoBundler({
      inputGraph: mutableBundleGraph,
      outputGraph: mutableBundleGraph,
      entries,
    });

    expectBundles(path.dirname(entryPath), outputBundleGraph, [
      {
        assets: ['dependency.js', 'test.js'],
      },
    ]);
  });

  it('can bundle multiple files with async dependencies', async () => {
    const entryPath = path.join(__dirname, 'test/test.js');
    await fsFixture(overlayFS, __dirname)`
      test
        dependency.js:
          export function something(name) { return 'hey ' + name; };
        async.js:
          export async function other() { return 5; };
        test.js:
          import {something} from './dependency';

          export async function main() {
            const {other} = await import('./async');
            return something('atlaspack') + ' explain it like I am ' + other();
          };
    `;

    const {mutableBundleGraph, entries} = await setupBundlerTest(entryPath);

    const {bundleGraph: outputBundleGraph} = monoBundler({
      inputGraph: mutableBundleGraph,
      outputGraph: mutableBundleGraph,
      entries,
    });

    expectBundles(path.dirname(entryPath), outputBundleGraph, [
      {
        assets: ['async.js', 'dependency.js', 'test.js'],
      },
    ]);
  });
});
