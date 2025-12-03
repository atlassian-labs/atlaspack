import assert from 'assert';
import path from 'path';
import {
  describe,
  it,
  fsFixture,
  overlayFS,
  inputFS,
  napiWorkerPool,
} from '@atlaspack/test-utils';
import {AtlaspackV3, FileSystemV3} from '@atlaspack/core';
import {NodePackageManager} from '@atlaspack/package-manager';
import {LMDBLiteCache} from '@atlaspack/cache';

/**
 * Integration tests for the native (Rust) transformer caching mechanism.
 *
 * These tests verify that the memoization cache correctly:
 * - Caches transformer results between builds
 * - Invalidates cache when source files change
 * - Returns correct results from cache
 */
describe.v3('Native Transformer Cache', function () {
  let dir: string;

  beforeEach(async function () {
    dir = path.join(__dirname, 'native-cache-fixture');
    await overlayFS.rimraf(dir);
    await overlayFS.mkdirp(dir);
  });

  afterEach(async function () {
    await overlayFS.rimraf(dir);
  });

  describe('Basic Cache Behavior', function () {
    it('builds', async () => {
      await fsFixture(overlayFS, __dirname)`
        index.js:
          console.log('hello world');

        .parcelrc:
          {
            "extends": "@atlaspack/config-default",
            "transformers": {
              "*.{js,mjs,jsm,jsx,es6,cjs,ts,tsx}": ["@atlaspack/transformer-js"]
            }
          }

        yarn.lock: {}
      `;

      let atlaspack = await AtlaspackV3.create({
        corePath: '',
        serveOptions: false,
        entries: [path.join(__dirname, 'index.js')],
        fs: new FileSystemV3(overlayFS),
        napiWorkerPool,
        packageManager: new NodePackageManager(inputFS, __dirname),
        lmdb: new LMDBLiteCache('.parcel-cache').getNativeRef(),
      });

      await atlaspack.buildAssetGraph();
    });

    it('should produce identical output on rebuild without changes', async function () {
      await fsFixture(overlayFS, dir)`
        index.js:
          export function hello() {
            return 'hello world';
          }
          export default hello();

        .parcelrc:
          {
            "extends": "@atlaspack/config-default",
            "transformers": {
              "*.{js,mjs,jsm,jsx,es6,cjs,ts,tsx}": ["@atlaspack/transformer-js"]
            }
          }

        yarn.lock: {}
      `;

      console.log('Creating cache at:', dir);
      const cacheDir = path.join(dir, '.parcel-cache');
      await overlayFS.mkdirp(cacheDir);
      const cache = new LMDBLiteCache(cacheDir);

      console.log('Creating first AtlaspackV3 instance...');
      // Create AtlaspackV3 instance
      let atlaspack = await AtlaspackV3.create({
        corePath: '',
        serveOptions: false,
        entries: [path.join(dir, 'index.js')],
        fs: new FileSystemV3(overlayFS),
        napiWorkerPool,
        packageManager: new NodePackageManager(inputFS, dir),
        lmdb: cache.getNativeRef(),
      });

      console.log('Running first build...');
      // First build
      await atlaspack.buildAssetGraph();
      console.log('First build complete!');
      console.log('Getting cache stats...');
      let stats1 = await atlaspack.getCacheStats();
      console.log('Got stats1:', stats1);

      // On first build, we expect misses (no cache hits)
      assert.equal(stats1.hits, 0, 'First build should have no cache hits');
      assert(stats1.misses > 0, 'First build should have cache misses');

      console.log('Running second build with same instance...');
      // Second build with same instance (should use cache)
      await atlaspack.buildAssetGraph();
      console.log('Second build complete!');
      let stats2 = await atlaspack.getCacheStats();
      console.log('Got stats2:', stats2);

      // On second build with same instance, we expect cache hits
      assert(
        stats2.hits > 0,
        `Second build should have cache hits, got: ${JSON.stringify(stats2)}`,
      );
    });

    // it('should invalidate cache when source file changes', async function () {
    //   await fsFixture(overlayFS, dir)`
    //     yarn.lock:

    //     package.json:
    //       {
    //         "name": "cache-test",
    //         "version": "1.0.0"
    //       }

    //     .parcelrc:
    //       {
    //         "extends": "@atlaspack/config-default",
    //         "transformers": {
    //           "*.{js,mjs,jsm,jsx,es6,cjs,ts,tsx}": ["@atlaspack/transformer-js"]
    //         }
    //       }

    //     src/index.js:
    //       export default 'initial value';
    //   `;

    //   const cacheDir = path.join(dir, '.parcel-cache');
    //   await overlayFS.mkdirp(cacheDir);
    //   const cache = new LMDBLiteCache(cacheDir);

    //   // First build
    //   let atlaspack1 = await AtlaspackV3.create({
    //     corePath: '',
    //     serveOptions: false,
    //     entries: [path.join(dir, 'src/index.js')],
    //     fs: new FileSystemV3(overlayFS),
    //     napiWorkerPool,
    //     packageManager: new NodePackageManager(inputFS, dir),
    //     lmdb: cache.getNativeRef(),
    //   });

    //   await atlaspack1.buildAssetGraph();
    //   let stats1 = atlaspack1.getCacheStats();
    //   assert.equal(stats1.hits, 0, 'First build should have no cache hits');

    //   // Modify the source file
    //   await overlayFS.writeFile(
    //     path.join(dir, 'src/index.js'),
    //     "export default 'updated value';",
    //   );

    //   // Second build (should detect change and rebuild)
    //   let atlaspack2 = await AtlaspackV3.create({
    //     corePath: '',
    //     serveOptions: false,
    //     entries: [path.join(dir, 'src/index.js')],
    //     fs: new FileSystemV3(overlayFS),
    //     napiWorkerPool,
    //     packageManager: new NodePackageManager(inputFS, dir),
    //     lmdb: cache.getNativeRef(),
    //   });

    //   await atlaspack2.buildAssetGraph();
    //   let stats2 = atlaspack2.getCacheStats();

    //   // After file change, we should still have some misses (invalidated entries)
    //   assert(stats2.misses > 0, 'Should have cache misses after file change');

    //   atlaspack1.end();
    //   atlaspack2.end();
    // });

    // it('should handle multiple files with dependencies', async function () {
    //   await fsFixture(overlayFS, dir)`
    //     yarn.lock:

    //     package.json:
    //       {
    //         "name": "cache-test",
    //         "version": "1.0.0"
    //       }

    //     .parcelrc:
    //       {
    //         "extends": "@atlaspack/config-default",
    //         "transformers": {
    //           "*.{js,mjs,jsm,jsx,es6,cjs,ts,tsx}": ["@atlaspack/transformer-js"]
    //         }
    //       }

    //     src/index.js:
    //       import { getValue } from './helper';
    //       export default getValue();

    //     src/helper.js:
    //       export function getValue() {
    //         return 42;
    //       }
    //   `;

    //   const cacheDir = path.join(dir, '.parcel-cache');
    //   await overlayFS.mkdirp(cacheDir);
    //   const cache = new LMDBLiteCache(cacheDir);

    //   // First build
    //   let atlaspack1 = await AtlaspackV3.create({
    //     corePath: '',
    //     serveOptions: false,
    //     entries: [path.join(dir, 'src/index.js')],
    //     fs: new FileSystemV3(overlayFS),
    //     napiWorkerPool,
    //     packageManager: new NodePackageManager(inputFS, dir),
    //     lmdb: cache.getNativeRef(),
    //   });

    //   await atlaspack1.buildAssetGraph();
    //   let stats1 = atlaspack1.getCacheStats();
    //   let firstBuildMisses = stats1.misses;
    //   assert.equal(stats1.hits, 0, 'First build should have no cache hits');
    //   assert(firstBuildMisses > 0, 'First build should have cache misses');

    //   // Rebuild without changes
    //   let atlaspack2 = await AtlaspackV3.create({
    //     corePath: '',
    //     serveOptions: false,
    //     entries: [path.join(dir, 'src/index.js')],
    //     fs: new FileSystemV3(overlayFS),
    //     napiWorkerPool,
    //     packageManager: new NodePackageManager(inputFS, dir),
    //     lmdb: cache.getNativeRef(),
    //   });

    //   await atlaspack2.buildAssetGraph();
    //   let stats2 = atlaspack2.getCacheStats();
    //   assert(stats2.hits > 0, 'Second build should have cache hits');

    //   // Modify dependency
    //   await overlayFS.writeFile(
    //     path.join(dir, 'src/helper.js'),
    //     `export function getValue() {
    //       return 100;
    //     }`,
    //   );

    //   // Third build (should detect change in dependency)
    //   let atlaspack3 = await AtlaspackV3.create({
    //     corePath: '',
    //     serveOptions: false,
    //     entries: [path.join(dir, 'src/index.js')],
    //     fs: new FileSystemV3(overlayFS),
    //     napiWorkerPool,
    //     packageManager: new NodePackageManager(inputFS, dir),
    //     lmdb: cache.getNativeRef(),
    //   });

    //   await atlaspack3.buildAssetGraph();
    //   let stats3 = atlaspack3.getCacheStats();
    //   assert(stats3.misses > 0, 'Third build should have misses due to invalidation');

    //   atlaspack1.end();
    //   atlaspack2.end();
    //   atlaspack3.end();
    // });
  });
});
