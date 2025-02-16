// @flow strict-local

import path from 'path';
import assert from 'assert';
import sinon from 'sinon';
import MutableBundleGraph from '@atlaspack/core/src/public/MutableBundleGraph.js';
// $FlowFixMe
import expect from 'expect';
import {
  bundle,
  expectBundles,
  fsFixture,
  overlayFS,
  run,
} from '@atlaspack/test-utils';

const invariant = assert;

describe('bundler-experimental', () => {
  let createBundleSpy;
  let createBundleGroupSpy;
  let createBundleReferenceSpy;
  let createAssetReferenceSpy;
  let internalizeAsyncDependencySpy;
  let addAssetToBundleSpy;
  let addBundleToBundleGroupSpy;

  beforeEach(() => {
    createBundleSpy = sinon.spy(MutableBundleGraph.prototype, 'createBundle');
    createBundleGroupSpy = sinon.spy(
      MutableBundleGraph.prototype,
      'createBundleGroup',
    );
    createBundleReferenceSpy = sinon.spy(
      MutableBundleGraph.prototype,
      'createBundleReference',
    );
    createAssetReferenceSpy = sinon.spy(
      MutableBundleGraph.prototype,
      'createAssetReference',
    );
    internalizeAsyncDependencySpy = sinon.spy(
      MutableBundleGraph.prototype,
      'internalizeAsyncDependency',
    );
    addAssetToBundleSpy = sinon.spy(
      MutableBundleGraph.prototype,
      'addAssetToBundle',
    );
    addBundleToBundleGroupSpy = sinon.spy(
      MutableBundleGraph.prototype,
      'addBundleToBundleGroup',
    );
  });

  afterEach(() => {
    createBundleSpy.restore();
    createBundleGroupSpy.restore();
    createBundleReferenceSpy.restore();
    createAssetReferenceSpy.restore();
    internalizeAsyncDependencySpy.restore();
    addAssetToBundleSpy.restore();
    addBundleToBundleGroupSpy.restore();
  });

  describe('parity tests', () => {
    const bundlers = [
      '@atlaspack/bundler-default',
      '@atlaspack/bundler-experimental',
    ];

    const graphs = new Map();
    bundlers.forEach((bundler) => {
      describe(`${bundler}`, () => {
        it('can bundle a single file into an output file', async () => {
          await fsFixture(overlayFS, __dirname)`
      bundler-experimental
        index.js:
          output(1234);

        package.json:
          {}
        yarn.lock:
          {}

        .parcelrc:
          {
            "extends": "@atlaspack/config-default",
            "bundler": ${JSON.stringify(bundler)}
          }
    `;

          const inputDir = path.join(__dirname, 'bundler-experimental');
          const b = await bundle(path.join(inputDir, 'index.js'), {
            inputFS: overlayFS,
            outputFS: overlayFS,
          });

          // $FlowFixMe
          expectBundles(inputDir, b, [
            {
              name: 'index.js',
              type: 'js',
              assets: ['index.js'],
            },
          ]);

          let output = null;
          await run(b, {
            output: (value) => {
              output = value;
            },
          });

          expect(output).toEqual(1234);
        });

        it('can bundle two files together', async () => {
          await fsFixture(overlayFS, __dirname)`
      bundler-experimental
        dependency.js:
          module.exports = () => 1234;
        index.js:
          const get = require('./dependency');
          output(get());

        package.json:
          {}
        yarn.lock:
          {}

        .parcelrc:
          {
            "extends": "@atlaspack/config-default",
            "bundler": ${JSON.stringify(bundler)}
          }
    `;

          const inputDir = path.join(__dirname, 'bundler-experimental');
          const b = await bundle(path.join(inputDir, 'index.js'), {
            inputFS: overlayFS,
            outputFS: overlayFS,
          });

          // $FlowFixMe
          expectBundles(inputDir, b, [
            {
              name: 'index.js',
              type: 'js',
              assets: ['dependency.js', 'index.js'],
            },
          ]);

          let output = null;
          await run(b, {
            output: (value) => {
              output = value;
            },
          });

          expect(output).toEqual(1234);
        });

        it('can bundle async splits', async () => {
          await fsFixture(overlayFS, __dirname)`
      bundler-experimental
        async.js:
          module.exports = () => 34;
        dependency.js:
          module.exports = () => 1200;
        index.js:
          const get = require('./dependency');
          output(import('./async').then((get2) => {
            return get() + get2();
          }));

        package.json:
          {}
        yarn.lock:
          {}

        .parcelrc:
          {
            "extends": "@atlaspack/config-default",
            "bundler": ${JSON.stringify(bundler)}
          }
    `;

          const inputDir = path.join(__dirname, 'bundler-experimental');
          const b = await bundle(path.join(inputDir, 'index.js'), {
            inputFS: overlayFS,
            outputFS: overlayFS,
          });

          const bundles = b.getBundles();
          assert.equal(bundles.length, 2);
          const bundle1 = bundles.find((bundle) =>
            bundle.getEntryAssets()[0].filePath.includes('index.js'),
          );
          const bundle2 = bundles.find((bundle) =>
            bundle.getEntryAssets()[0].filePath.includes('async.js'),
          );
          invariant(bundle1);
          invariant(bundle2);

          const bundleGroupsForIndex =
            b.getBundleGroupsContainingBundle(bundle1);
          assert.equal(bundleGroupsForIndex.length, 1);
          const bundleGroupsForAsync =
            b.getBundleGroupsContainingBundle(bundle2);
          assert.equal(bundleGroupsForAsync.length, 1);
          // loaded in different bundle groups
          assert.notStrictEqual(
            bundleGroupsForIndex[0],
            bundleGroupsForAsync[0],
          );

          assert.equal(createBundleSpy.callCount, 2);
          assert(
            createBundleSpy
              .getCalls()[0]
              .args[0].entryAsset.filePath.includes('index.js'),
          );
          assert(
            createBundleSpy
              .getCalls()[1]
              .args[0].entryAsset.filePath.includes('async.js'),
          );

          assert.equal(createBundleGroupSpy.callCount, 2);
          assert.equal(createBundleReferenceSpy.callCount, 0);
          assert.equal(createAssetReferenceSpy.callCount, 0);
          assert.equal(internalizeAsyncDependencySpy.callCount, 0);
          assert.deepStrictEqual(
            addAssetToBundleSpy
              .getCalls()
              .map((call) => [
                path.relative(inputDir, call.args[0].filePath),
                path.relative(
                  inputDir,
                  call.args[1].getEntryAssets()[0].filePath,
                ),
              ]),
            [
              ['index.js', 'index.js'],
              ['dependency.js', 'index.js'],
              ['async.js', 'async.js'],
            ],
          );

          // $FlowFixMe
          expectBundles(inputDir, b, [
            {
              type: 'js',
              assets: ['async.js'],
            },
            {
              type: 'js',
              name: 'index.js',
              assets: ['dependency.js', 'index.js'],
            },
          ]);

          let output = null;
          graphs.set(bundler, b);

          await run(b, {
            output: (value) => {
              output = value;
            },
          });

          expect(await output).toEqual(1234);
        });

        it('can bundle two level deep async splits', async () => {
          // TMP diff to test shared bundles
          await fsFixture(overlayFS, __dirname)`
      bundler-experimental
        async.js:
          module.exports = () => 10;

        page1.js:
          module.exports = async () => {
            const get = await import('./async');
            return get();
          };

        page2.js:
          module.exports = async () => {
            const get = await import('./async');
            return get();
          };

        index.js:
          async function run() {
            const page1 = await import('./page1');
            const page2 = await import('./page2');
            return await page1() + await page2();
          }
          output(run());

        package.json:
          {
             "@atlaspack/bundler-default": {
                "minBundleSize": 0
             }
          }

        yarn.lock:
          {}

        .parcelrc:
          {
            "extends": "@atlaspack/config-default",
            "bundler": ${JSON.stringify(bundler)}
          }
    `;

          const inputDir = path.join(__dirname, 'bundler-experimental');
          const b = await bundle(path.join(inputDir, 'index.js'), {
            inputFS: overlayFS,
            outputFS: overlayFS,
          });

          // $FlowFixMe
          expectBundles(inputDir, b, [
            {
              type: 'js',
              assets: ['async.js'],
            },
            {
              type: 'js',
              assets: ['index.js'],
            },
            {
              type: 'js',
              assets: ['page1.js'],
            },
            {
              type: 'js',
              assets: ['page2.js'],
            },
          ]);

          let output = null;
          graphs.set(bundler, b);

          await run(b, {
            output: (value) => {
              output = value;
            },
          });

          expect(await output).toEqual(20);
        });

        it('can bundle async splits without duplicating parent dependencies', async () => {
          // TMP diff to test shared bundles
          await fsFixture(overlayFS, __dirname)`
      bundler-experimental
        async.js:
          const get = require('./dependency');
          module.exports = () => 1 + get();

        dependency.js:
          module.exports = () => 1;

        page1.js:
          const get = require('./dependency');
          module.exports = async () => {
            const get2 = await import('./async');
            return get() + get2();
          };

        page2.js:
          const get = require('./dependency');
          module.exports = async () => {
            const get2 = await import('./async');
            return get() + get2();
          };

        index.js:
          async function run() {
            const page1 = await import('./page1');
            const page2 = await import('./page2');
            return await page1() + await page2();
          }
          output(run());

        package.json:
          {
             "@atlaspack/bundler-default": {
                "minBundleSize": 0
             }
          }

        yarn.lock:
          {}

        .parcelrc:
          {
            "extends": "@atlaspack/config-default",
            "bundler": ${JSON.stringify(bundler)}
          }
    `;

          const inputDir = path.join(__dirname, 'bundler-experimental');
          const b = await bundle(path.join(inputDir, 'index.js'), {
            inputFS: overlayFS,
            outputFS: overlayFS,
          });

          const bundles = b.getBundles();
          assert.equal(bundles.length, 5);
          const entryBundle = bundles.find((bundle) =>
            bundle.getEntryAssets()[0]?.filePath.includes('index.js'),
          );
          const page1Bundle = bundles.find((bundle) =>
            bundle.getEntryAssets()[0]?.filePath.includes('page1.js'),
          );
          const page2Bundle = bundles.find((bundle) =>
            bundle.getEntryAssets()[0]?.filePath.includes('page2.js'),
          );
          const sharedBundle = bundles.find((bundle) => {
            let found = false;
            bundle.traverseAssets((asset) => {
              if (asset.filePath.includes('dependency.js')) {
                found = true;
              }
            });
            return found;
          });
          const asyncBundle = bundles.find((bundle) =>
            bundle.getEntryAssets()[0]?.filePath.includes('async.js'),
          );
          invariant(entryBundle);
          invariant(asyncBundle);
          invariant(sharedBundle);
          invariant(page1Bundle);
          invariant(page2Bundle);

          const bundleGroupsForIndex =
            b.getBundleGroupsContainingBundle(entryBundle);
          const bundleGroupsForAsync =
            b.getBundleGroupsContainingBundle(asyncBundle);
          const bundleGroupsForPage1 =
            b.getBundleGroupsContainingBundle(page1Bundle);
          const bundleGroupsForPage2 =
            b.getBundleGroupsContainingBundle(page2Bundle);
          assert.equal(bundleGroupsForPage1.length, 1);
          assert.equal(bundleGroupsForPage2.length, 1);
          assert.equal(bundleGroupsForIndex.length, 1);
          assert.equal(bundleGroupsForAsync.length, 1);
          // assert.equal(bundleGroupsForShared.length, 2);
          assert.notStrictEqual(
            bundleGroupsForIndex[0],
            bundleGroupsForAsync[0],
          );

          // assert.deepStrictEqual(bundleGroupsForShared, [
          //   ...bundleGroupsForPage2,
          //   ...bundleGroupsForPage1,
          // ]);
          assert.equal(createBundleSpy.callCount, 5);
          // assert.equal(addBundleToBundleGroupSpy.callCount, 4);

          // $FlowFixMe
          expectBundles(inputDir, b, [
            {
              type: 'js',
              assets: ['async.js'],
            },
            {
              type: 'js',
              assets: ['dependency.js'],
            },
            {
              type: 'js',
              assets: ['index.js'],
            },
            {
              type: 'js',
              assets: ['page1.js'],
            },
            {
              type: 'js',
              assets: ['page2.js'],
            },
          ]);

          let output = null;
          graphs.set(bundler, b);

          await run(b, {
            output: (value) => {
              output = value;
            },
          });

          expect(await output).toEqual(6);
        });
      });
    });
  });
});
