import assert from 'assert';
import childProcess from 'child_process';
import nodeFS from 'fs';
import path, {join} from 'path';
import {
  assertBundles,
  bundler,
  describe,
  fsFixture,
  getNextBuild,
  it,
  run,
  assertBundleTree,
  nextBundle,
  ncp,
  inputFS as fs,
  sleep,
  symlinkPrivilegeWarning,
  outputFS,
  overlayFS,
  inputFS,
} from '@atlaspack/test-utils';
import {symlinkSync} from 'fs';
import tempy from 'tempy';
import {
  AsyncSubscription,
  BundleGraph,
  PackagedBundle,
} from '@atlaspack/types-internal';

const inputDir = path.join(__dirname, '/watcher');
const distDir = path.join(inputDir, 'dist');

describe('watcher', function () {
  let subscription: AsyncSubscription;
  afterEach(async () => {
    if (subscription) {
      await subscription.unsubscribe();
    }
    subscription = null;
  });

  it('should rebuild on source file change', async function () {
    await outputFS.mkdirp(inputDir);
    await outputFS.writeFile(
      path.join(inputDir, '/index.js'),
      'module.exports = "hello"',
      {encoding: 'utf8'},
    );
    let b = bundler(path.join(inputDir, '/index.js'), {inputFS: overlayFS});
    subscription = await b.watch();
    let buildEvent = await getNextBuild(b);
    assert(buildEvent.type === 'buildSuccess');

    let output = await run(buildEvent.bundleGraph);
    assert.equal(output, 'hello');

    await outputFS.writeFile(
      path.join(inputDir, '/index.js'),
      'module.exports = "something else"',
      {encoding: 'utf8'},
    );
    buildEvent = await getNextBuild(b);
    assert(buildEvent.type === 'buildSuccess');

    output = await run(buildEvent.bundleGraph);
    assert.equal(output, 'something else');
  });

  it('should rebuild on a source file change after a failed transformation', async () => {
    await outputFS.mkdirp(inputDir);
    await outputFS.writeFile(
      path.join(inputDir, '/index.js'),
      'syntax\\error',
      {encoding: 'utf8'},
    );
    let b = bundler(path.join(inputDir, '/index.js'), {inputFS: overlayFS});
    subscription = await b.watch();
    let buildEvent = await getNextBuild(b);
    assert.equal(buildEvent.type, 'buildFailure');
    await outputFS.writeFile(
      path.join(inputDir, '/index.js'),
      'module.exports = "hello"',
      {encoding: 'utf8'},
    );
    buildEvent = await getNextBuild(b);
    assert(buildEvent.type === 'buildSuccess');

    let output = await run(buildEvent.bundleGraph);

    assert.equal(output, 'hello');
  });

  it.v2('should rebuild on a config file change', async function () {
    let inDir = path.join(__dirname, 'integration/parcelrc-custom');
    let outDir = path.join(inDir, 'dist');

    await ncp(path.join(__dirname, 'integration/parcelrc-custom'), inDir);
    await ncp(
      path.dirname(require.resolve('@atlaspack/config-default')),
      path.join(inDir, 'node_modules', '@atlaspack', 'config-default'),
    );
    let copyPath = path.join(inDir, 'configCopy');
    let configPath = path.join(inDir, '.parcelrc');
    let b = bundler(path.join(inDir, 'index.js'), {
      inputFS: overlayFS,
      targets: {
        main: {
          distDir: outDir,
        },
      },
    });
    subscription = await b.watch();
    await getNextBuild(b);
    let distFile = await outputFS.readFile(
      path.join(outDir, 'index.js'),
      'utf8',
    );
    assert(distFile.includes('() => null'));
    await outputFS.copyFile(copyPath, configPath);
    await getNextBuild(b);
    distFile = await outputFS.readFile(path.join(outDir, 'index.js'), 'utf8');
    assert(distFile.includes('TRANSFORMED CODE'));
  });

  it('should rebuild properly when a dependency is removed', async function () {
    await ncp(path.join(__dirname, 'integration/babel-default'), inputDir);

    let b = bundler(path.join(inputDir, 'index.js'), {
      inputFS: overlayFS,
      targets: {
        main: {
          engines: {
            node: '^8.0.0',
          },
          distDir,
        },
      },
    });

    subscription = await b.watch();
    let buildEvent = await getNextBuild(b);
    assert.equal(buildEvent.type, 'buildSuccess');
    let distFile = await outputFS.readFile(
      path.join(distDir, 'index.js'),
      'utf8',
    );
    assert(distFile.includes('Foo'));
    await outputFS.writeFile(
      path.join(inputDir, 'index.js'),
      'console.log("no more dependencies")',
    );
    await getNextBuild(b);
    distFile = await outputFS.readFile(path.join(distDir, 'index.js'), 'utf8');
    assert(!distFile.includes('Foo'));
  });

  it.skip('should re-generate bundle tree when files change', async function () {
    await ncp(path.join(__dirname, '/integration/dynamic-hoist'), inputDir);

    let b = bundler(path.join(inputDir, '/index.js'), {watch: true});
    let bundle = await b.bundle();

    await assertBundleTree(bundle, {
      name: 'index.js',
      assets: [
        'index.js',
        'common.js',
        'common-dep.js',
        'bundle-loader.js',
        'bundle-url.js',
        'js-loader.js',
      ],
      childBundles: [
        {
          assets: ['a.js'],
          childBundles: [
            {
              type: 'map',
            },
          ],
        },
        {
          assets: ['b.js'],
          childBundles: [
            {
              type: 'map',
            },
          ],
        },
        {
          type: 'map',
        },
      ],
    });

    let output = await run(bundle);
    assert.equal(await output(), 7);

    // change b.js so that it no longer depends on common.js.
    // This should cause common.js and dependencies to no longer be hoisted to the root bundle.
    await sleep(100);
    fs.writeFile(path.join(inputDir, '/b.js'), 'module.exports = 5;');

    bundle = await nextBundle(b);
    await assertBundleTree(bundle, {
      name: 'index.js',
      assets: ['index.js', 'bundle-loader.js', 'bundle-url.js', 'js-loader.js'],
      childBundles: [
        {
          assets: ['a.js', 'common.js', 'common-dep.js'],
          childBundles: [
            {
              type: 'map',
            },
          ],
        },
        {
          assets: ['b.js'],
          childBundles: [
            {
              type: 'map',
            },
          ],
        },
        {
          type: 'map',
        },
      ],
    });

    output = await run(bundle);
    assert.equal(await output(), 8);
  });

  it.skip('should only re-package bundles that changed', async function () {
    await ncp(path.join(__dirname, '/integration/dynamic-hoist'), inputDir);
    let b = bundler(path.join(inputDir, '/index.js'), {watch: true});

    await b.bundle();
    let mtimes = (await fs.readdir(path.join(__dirname, '/dist'))).map(
      (f) =>
        (nodeFS.statSync(path.join(__dirname, '/dist/', f)).mtime.getTime() /
          1000) |
        0,
    );

    await sleep(1100); // mtime only has second level precision
    fs.writeFile(
      path.join(inputDir, '/b.js'),
      'module.exports = require("./common")',
    );

    await nextBundle(b);
    let newMtimes = (await fs.readdir(path.join(__dirname, '/dist'))).map(
      (f) =>
        (nodeFS.statSync(path.join(__dirname, '/dist/', f)).mtime.getTime() /
          1000) |
        0,
    );
    assert.deepEqual(mtimes.sort().slice(0, 2), newMtimes.sort().slice(0, 2));
    assert.notEqual(mtimes[mtimes.length - 1], newMtimes[newMtimes.length - 1]);
  });

  it.skip('should unload assets that are orphaned', async function () {
    await ncp(path.join(__dirname, '/integration/dynamic-hoist'), inputDir);
    let b = bundler(path.join(inputDir, '/index.js'), {watch: true});

    let bundle = await b.bundle();
    await assertBundleTree(bundle, {
      name: 'index.js',
      assets: [
        'index.js',
        'common.js',
        'common-dep.js',
        'bundle-loader.js',
        'bundle-url.js',
        'js-loader.js',
      ],
      childBundles: [
        {
          assets: ['a.js'],
          childBundles: [
            {
              type: 'map',
            },
          ],
        },
        {
          assets: ['b.js'],
          childBundles: [
            {
              type: 'map',
            },
          ],
        },
        {
          type: 'map',
        },
      ],
    });

    let output = await run(bundle);
    assert.equal(await output(), 7);

    assert(b.loadedAssets.has(path.join(inputDir, '/common-dep.js')));

    // Get rid of common-dep.js
    await sleep(100);
    fs.writeFile(path.join(inputDir, '/common.js'), 'module.exports = 5;');

    bundle = await nextBundle(b);
    await assertBundleTree(bundle, {
      name: 'index.js',
      assets: [
        'index.js',
        'common.js',
        'bundle-loader.js',
        'bundle-url.js',
        'js-loader.js',
      ],
      childBundles: [
        {
          assets: ['a.js'],
          childBundles: [
            {
              type: 'map',
            },
          ],
        },
        {
          assets: ['b.js'],
          childBundles: [
            {
              type: 'map',
            },
          ],
        },
        {
          type: 'map',
        },
      ],
    });

    output = await run(bundle);
    assert.equal(await output(), 13);

    assert(!b.loadedAssets.has(path.join(inputDir, 'common-dep.js')));
  });

  it.skip('should recompile all assets when a config file changes', async function () {
    await ncp(path.join(__dirname, '/integration/babel'), inputDir);
    let b = bundler(path.join(inputDir, 'index.js'), {watch: true});

    await b.bundle();
    let file = await fs.readFile(
      path.join(__dirname, '/dist/index.js'),
      'utf8',
    );
    assert(!file.includes('function Foo'));
    assert(!file.includes('function Bar'));

    // Change babelrc, should recompile both files
    let babelrc = JSON.parse(
      await fs.readFile(path.join(inputDir, '/.babelrc'), 'utf8'),
    );
    babelrc.presets[0][1].targets.browsers.push('IE >= 11');

    await sleep(100);
    fs.writeFile(path.join(inputDir, '/.babelrc'), JSON.stringify(babelrc));

    await nextBundle(b);
    file = await fs.readFile(path.join(__dirname, '/dist/index.js'), 'utf8');
    assert(file.includes('function Foo'));
    assert(file.includes('function Bar'));
  });

  it.skip('should rebuild if the file behind a symlink changes', async function () {
    await ncp(
      path.join(__dirname, '/integration/commonjs-with-symlinks/'),
      inputDir,
    );

    try {
      // Create the symlink here to prevent cross platform and git issues
      symlinkSync(
        path.join(inputDir, 'local.js'),
        path.join(inputDir, 'src/symlinked_local.js'),
      );

      let b = bundler(path.join(inputDir, '/src/index.js'), {
        watch: true,
      });

      let bundle = await b.bundle();
      let output = await run(bundle);

      assert.equal(output(), 3);

      await sleep(100);
      fs.writeFile(
        path.join(inputDir, '/local.js'),
        'exports.a = 5; exports.b = 5;',
      );

      bundle = await nextBundle(b);
      output = await run(bundle);
      assert.equal(output(), 10);
    } catch (e: any) {
      if (e.code == 'EPERM') {
        symlinkPrivilegeWarning();
        this.skip();
      } else {
        throw e;
      }
    }
  });

  it('should add and remove necessary runtimes to bundles', async () => {
    await ncp(path.join(__dirname, 'integration/dynamic'), inputDir);

    let indexPath = path.join(inputDir, 'index.js');

    let b = bundler(indexPath, {inputFS: overlayFS});
    let bundleGraph: BundleGraph<PackagedBundle>;
    subscription = await b.watch((_err, event) => {
      assert(event && event.type === 'buildSuccess');
      bundleGraph = event.bundleGraph;
    });
    await getNextBuild(b);

    assert(bundleGraph, 'Expected bundleGraph to be defined');
    assertBundles(bundleGraph, [
      {
        name: 'index.js',
        assets: ['index.js', 'bundle-url.js', 'cacheLoader.js', 'js-loader.js'],
      },
      {assets: ['local.js']},
    ]);

    await outputFS.writeFile(path.join(inputDir, 'other.js'), '');
    await outputFS.writeFile(
      indexPath,
      (await outputFS.readFile(indexPath, 'utf8')) +
        "\nimport('./other.js');\n",
    );

    await getNextBuild(b);
    assert(bundleGraph, 'Expected bundleGraph to be defined');

    assertBundles(bundleGraph, [
      {
        name: 'index.js',
        assets: ['index.js', 'bundle-url.js', 'cacheLoader.js', 'js-loader.js'],
      },
      {assets: ['local.js']},
      {assets: ['other.js']},
    ]);

    await outputFS.writeFile(indexPath, '');

    await getNextBuild(b);
    assert(bundleGraph, 'Expected bundleGraph to be defined');

    assertBundles(bundleGraph, [
      {
        name: 'index.js',
        assets: ['index.js'],
      },
    ]);
  });

  it('should rebuild if a missing file is added', async function () {
    await outputFS.mkdirp(inputDir);
    await outputFS.writeFile(
      path.join(inputDir, '/index.js'),
      'import {other} from "./other";\nexport default other;',
    );

    let b = bundler(path.join(inputDir, 'index.js'), {inputFS: overlayFS});
    subscription = await b.watch();
    let buildEvent = await getNextBuild(b);
    assert.equal(buildEvent.type, 'buildFailure');

    await outputFS.writeFile(
      path.join(inputDir, '/other.js'),
      'export const other = 2;',
    );

    buildEvent = await getNextBuild(b);
    assert(buildEvent.type === 'buildSuccess');

    let res = await run(buildEvent.bundleGraph);
    assert.equal(res.default, 2);
  });

  describe('watcher tests', () => {
    let hasWatchman = false;
    try {
      childProcess.execSync('watchman --version');
      hasWatchman = true;
    } catch (e: any) {
      hasWatchman = false;
    }

    it('must have watchman installed for our test suite to work', () => {
      assert(
        hasWatchman,
        'watchman must be installed for our test suite to work',
      );
    });

    if (hasWatchman) {
      it('watchman - picks-up changes in a directory', async () => {
        const tempDir = tempy.directory();
        await nodeFS.promises.mkdir(tempDir, {recursive: true});

        await fs.writeSnapshot(tempDir, path.join(tempDir, 'snapshot.txt'), {
          backend: 'watchman',
        });
        await fs.writeFile(path.join(tempDir, 'test.js'), 'export default 4');

        const events = (
          await fs.getEventsSince(tempDir, path.join(tempDir, 'snapshot.txt'), {
            backend: 'watchman',
          })
        ).map((e) => e.path);
        events.sort((a, b) => a.localeCompare(b));

        assert.deepEqual(events, [
          path.join(tempDir, 'snapshot.txt'),
          path.join(tempDir, 'test.js'),
        ]);
      });
    }

    if (process.platform === 'darwin') {
      it.skip('macOS - fs-events - fails to pick-up changes in a directory', async () => {
        const tempDir = tempy.directory();
        await nodeFS.promises.mkdir(tempDir, {recursive: true});

        await fs.writeSnapshot(tempDir, path.join(tempDir, 'snapshot.txt'), {
          backend: 'fs-events',
        });
        await fs.writeFile(path.join(tempDir, 'test.js'), 'export default 4');

        const events = await fs.getEventsSince(
          tempDir,
          path.join(tempDir, 'snapshot.txt'),
          {
            backend: 'fs-events',
          },
        );
        events.sort((a, b) => a.path.localeCompare(b.path));

        assert.deepEqual(events, []); // <--- ⚠️⚠️⚠️ This is wrong
      });
    }

    if (process.platform === 'linux') {
      it('linux - inotify - picks-up changes in a directory', async () => {
        const tempDir = tempy.directory();
        await nodeFS.promises.mkdir(tempDir, {recursive: true});

        await fs.writeSnapshot(tempDir, path.join(tempDir, 'snapshot.txt'), {
          backend: 'inotify',
        });
        await fs.writeFile(path.join(tempDir, 'test.js'), 'export default 4');

        const events = (
          await fs.getEventsSince(tempDir, path.join(tempDir, 'snapshot.txt'), {
            backend: 'inotify',
          })
        ).map((e) => e.path);
        events.sort((a, b) => a.localeCompare(b));

        assert.deepEqual(events, [
          path.join(tempDir, 'snapshot.txt'),
          path.join(tempDir, 'test.js'),
        ]);
      });
    }
  });

  describe('incremental rebuild loop', function () {
    const dir = join(__dirname, 'tmp');

    beforeEach(async function () {
      await inputFS.rimraf(dir);
    });

    afterEach(async function () {
      await inputFS.rimraf(dir);
    });

    // Regression test for the infinite rebuild loop caused by side_effects mismatch.
    // When package.json declares "sideEffects": false but a transformer overrides
    // asset.sideEffects = true, the incremental rebuild creates an AssetRequest
    // with a different ID than the original. The original invalidated node becomes
    // orphaned in invalid_nodes, causing respond_to_fs_events to permanently
    // return true and triggering an infinite rebuild loop.
    it('should not enter infinite rebuild loop when invalidated node is orphaned', async function () {
      this.timeout(30000);

      let outDir = path.join(dir, 'dist');

      // package.json declares "sideEffects": false, but the custom transformer
      // overrides asset.sideEffects = true. This mismatch causes the
      // incremental AssetRequest to have a different ID than the original,
      // orphaning the original invalidated node.
      await fsFixture(inputFS, dir)`
          watcher-side-effects-rebuild-loop
            package.json:
              {
                "name": "watcher-side-effects-test",
                "sideEffects": false
              }

            .parcelrc:
              {
                "extends": "@atlaspack/config-default",
                "transformers": {
                  "*.js": ["atlaspack-transformer-set-side-effects", "..."]
                }
              }

            index.js:
              export const value = "hello";

            node_modules/atlaspack-transformer-set-side-effects/package.json:
              {
                "name": "atlaspack-transformer-set-side-effects",
                "version": "1.0.0",
                "main": "index.js"
              }

            node_modules/atlaspack-transformer-set-side-effects/index.js:
              const Transformer = require('@atlaspack/plugin').Transformer;
              module.exports = new Transformer({
                transform({asset}) {
                  asset.sideEffects = true;
                  return [asset];
                },
              });
        `;

      let b = bundler(
        path.join(dir, 'watcher-side-effects-rebuild-loop/index.js'),
        {
          inputFS: inputFS,
          outputFS: inputFS,
          config: path.join(dir, 'watcher-side-effects-rebuild-loop/.parcelrc'),
          targets: {
            main: {
              distDir: outDir,
            },
          },
        },
      );

      // Track build count to detect infinite loop
      let buildCount = 0;
      subscription = await b.watch((_err, event) => {
        buildCount++;
      });

      // Wait for initial build
      let buildEvent = await getNextBuild(b);
      assert.equal(
        buildEvent.type,
        'buildSuccess',
        buildEvent.type === 'buildFailure'
          ? `Initial build failed: ${(buildEvent as any).diagnostics?.map((d: any) => d.message).join('; ')}`
          : 'Initial build should succeed',
      );

      // Edit the entry file to trigger a rebuild.
      await inputFS.writeFile(
        path.join(dir, 'watcher-side-effects-rebuild-loop/index.js'),
        'export const value = "world";',
      );

      // Wait for rebuild
      buildEvent = await getNextBuild(b);
      assert.equal(buildEvent.type, 'buildSuccess', 'Rebuild should succeed');

      // Give time for any extra rebuilds to occur.
      // If the bug is present, the orphaned invalid node causes
      // respond_to_fs_events to return true even when no events match
      // invalidation entries, triggering rebuilds.
      let buildCountAfterRebuild = buildCount;
      await sleep(3000);

      assert.equal(
        buildCount,
        buildCountAfterRebuild,
        `Expected no additional builds after the rebuild, ` +
          `but ${buildCount - buildCountAfterRebuild} extra build(s) occurred. ` +
          `The orphaned invalid node causes respond_to_fs_events ` +
          `to permanently return true, triggering rebuilds.`,
      );
    });
  });
});
