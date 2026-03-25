import assert from 'assert';
import {join} from 'path';

import {
  AtlaspackV3,
  FileSystemV3,
  Atlaspack,
  createWorkerFarm,
} from '@atlaspack/core';
import {NodePackageManager} from '@atlaspack/package-manager';
import stripAnsi from 'strip-ansi';
import sinon from 'sinon';
import {
  describe,
  bundler,
  fsFixture,
  inputFS,
  it,
  overlayFS,
  outputFS,
  bundle,
  napiWorkerPool,
} from '@atlaspack/test-utils';
import {LMDBLiteCache} from '@atlaspack/cache';
import type {InitialAtlaspackOptions} from '@atlaspack/types';

async function assertOutputIsIdentical(
  entry: string,
  options?: InitialAtlaspackOptions,
) {
  let bundlesV3 = await bundle(entry, {
    ...options,
    inputFS: overlayFS,
  }).then((b) => b.getBundles());

  let bundlesV2 = await bundle(entry, {
    ...options,
    inputFS: overlayFS,
    featureFlags: {
      atlaspackV3: false,
    },
  }).then((b) => b.getBundles());

  assert.equal(bundlesV3.length, bundlesV2.length);

  for (let i = 0; i < bundlesV2.length; i++) {
    let v2Code = await outputFS.readFile(bundlesV2[i].filePath, 'utf8');
    let v3Code = await outputFS.readFile(bundlesV3[i].filePath, 'utf8');

    assert.equal(v3Code, v2Code);
  }
}

function mockStdio() {
  const setup = (stream: any) => {
    let output = '';
    let stub = sinon.stub(stream, 'write').callsFake((chunk, ...args) => {
      output += stripAnsi(chunk.toString());
      if (process.env.DEBUG_TERMINAL_OUTPUT)
        stub.wrappedMethod.apply(stream, [chunk, ...args]);
      if (typeof args[args.length - 1] === 'function') args[args.length - 1]();
      return true;
    });
    return {getOutput: () => output, stub};
  };

  const stdout = setup(process.stdout);
  const stderr = setup(process.stderr);

  return {
    getStdout: stdout.getOutput,
    getStderr: stderr.getOutput,
    restore: () => {
      stdout.stub.restore();
      stderr.stub.restore();
    },
  };
}

describe.v3('AtlaspackV3', function () {
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
      env: process.env,
      entries: [join(__dirname, 'index.js')],
      fs: new FileSystemV3(overlayFS),
      napiWorkerPool,
      packageManager: new NodePackageManager(inputFS, __dirname),
      lmdb: new LMDBLiteCache('.parcel-cache').getNativeRef(),
    });

    await atlaspack.buildAssetGraph();
  });

  it('should map dependencies to assets', async () => {
    await fsFixture(overlayFS, __dirname)`
        dependencies
          library.ts:
            export default 'library';
          index.ts:
            import library from './library';
            sideEffectNoop(library);
          index.html:
            <script type="module" src="./index.ts" />
      `;

    let bundleGraph = await bundle(join(__dirname, 'dependencies/index.html'), {
      inputFS: overlayFS,
      defaultTargetOptions: {
        shouldScopeHoist: true,
      },
    });

    let jsBundle = bundleGraph.getBundles().find((b) => b.type === 'js');

    let indexAsset;
    jsBundle?.traverseAssets((asset: Asset) => {
      if (asset.filePath.includes('index.ts')) {
        indexAsset = asset;
      }
    });

    assert.deepEqual(
      indexAsset?.getDependencies().map((dep: Dependency) => dep.specifier),
      ['./library'],
    );
  });

  describe('should mirror V2 output', () => {
    it('with scope hoisting enabled', async () => {
      await fsFixture(overlayFS, __dirname)`
        scope-hoist
          node_modules/library/named.js:
            export default function namedFunction(arg) {
              return arg;
            }
          node_modules/library/index.js:
            import namedFunction from './named.js';
            export {namedFunction};
          node_modules/library/package.json:
            {"sideEffects": false}
          index.js:
            import {namedFunction} from 'library';
            sideEffectNoop(namedFunction(''));
          index.html:
            <script type="module" src="./index.js" />
      `;

      await assertOutputIsIdentical(join(__dirname, 'scope-hoist/index.html'), {
        defaultTargetOptions: {
          shouldScopeHoist: true,
        },
      });
    });

    it('with Assets that change type', async () => {
      await fsFixture(overlayFS, __dirname)`
        type-change
          name.json:
            {"name": "fred"}
          index.js:
            import * as json from './name.json';
            sideEffectNoop(json.name);
          index.html:
            <script type="module" src="./index.js" />
      `;

      await assertOutputIsIdentical(join(__dirname, 'type-change/index.html'));
    });

    it('with dynamic resolver code', async () => {
      await fsFixture(overlayFS, __dirname)`
        resolver-code
          index.js:
            import theGlobal from 'theGlobal';
            sideEffectNoop(theGlobal)
          index.html:
            <script type="module" src="./index.js" />
          package.json:
            {
              "alias": {
                "theGlobal": {
                  "global": "MY_GLOBAL"
                }
              }
            }
          yarn.lock:
      `;

      await assertOutputIsIdentical(
        join(__dirname, 'resolver-code/index.html'),
      );
    });

    it('with CSS modules', async () => {
      await fsFixture(overlayFS, __dirname)`
        css-modules
          css.module.css:
            .composed {
              background: pink;
            }
            .foo {
              composes: composed;
              color: white;
            }
          index.js:
            import {foo} from './css.module.css';
            sideEffectNoop(foo)
          index.html:
            <script type="module" src="./index.js" />
      `;

      await assertOutputIsIdentical(join(__dirname, 'css-modules/index.html'));
    });
  });

  // This test is a bit weird in that if it "fails" it will hang and stop Mocha from exiting.
  // I'm not sure there's actually any way to test for that any other way.
  it('cleanly shuts down when used via the Atlaspack API', async () => {
    await fsFixture(overlayFS, __dirname)`
      shutdown
        index.js:
          console.log('hello world');

        yarn.lock: {}
    `;

    let workerFarm = createWorkerFarm({
      maxConcurrentWorkers: 0,
      useLocalWorker: true,
    });
    try {
      let atlaspack = new Atlaspack({
        workerFarm,
        entries: [join(__dirname, 'shutdown/index.js')],
        inputFS: overlayFS,
        outputFS: overlayFS,
        config: '@atlaspack/config-default',
        shouldDisableCache: true,
        featureFlags: {
          atlaspackV3: true,
        },
      });
      const buildResult = await atlaspack.run();
      assert.equal(buildResult.type, 'buildSuccess');
    } finally {
      // We clean this one up because we created it
      await workerFarm.end();
    }
  });

  describe('featureFlags', () => {
    it('should not throw if feature flag is bool', async () => {
      await assert.doesNotReject(() =>
        AtlaspackV3.create({
          corePath: join(__dirname, '..', '..', 'core'),
          serveOptions: false,
          env: {},
          entries: [join(__dirname, 'integration', 'tokens', 'index.js')],
          fs: new FileSystemV3(overlayFS),
          lmdb: new LMDBLiteCache('.parcel-cache').getNativeRef(),
          napiWorkerPool,
          featureFlags: {
            testFlag: true,
          },
        }),
      );
    });

    it('should not throw if feature flag is string', async () => {
      await assert.doesNotReject(() =>
        AtlaspackV3.create({
          corePath: join(__dirname, '..', '..', 'core'),
          serveOptions: false,
          env: {},
          entries: [join(__dirname, 'integration', 'tokens', 'index.js')],
          fs: new FileSystemV3(overlayFS),
          lmdb: new LMDBLiteCache('.parcel-cache').getNativeRef(),
          napiWorkerPool,
          featureFlags: {
            testFlag: 'testFlagValue',
          },
        }),
      );
    });
  });

  // Regression test for V3's TypeScript/Rust interop: BitFlags conversion for packageConditions
  // When a CSS file imports another CSS file with a custom JS resolver, the Rust CSS
  // transformer sets package_conditions to ExportsCondition::STYLE (bitflags number 4096).
  // This test ensures packageConditions are correctly converted from Rust bitflags (numbers)
  // to TypeScript arrays in the Dependency constructor.
  it('should convert packageConditions from bitflags to array', async () => {
    const dir = join(__dirname, 'tmp', 'v3-css-style-condition');
    await inputFS.rimraf(dir);
    await inputFS.mkdirp(dir);

    await fsFixture(inputFS, dir)`
        package.json:
          {
            "name": "v3-css-style-condition-test"
          }

        .parcelrc:
          {
            "extends": "@atlaspack/config-default",
            "resolvers": ["./custom-resolver.js"]
          }

        yarn.lock:

        index.js:
          import './main.css';

        main.css:
          @import './variables.css';

          body {
            background: var(--bg-color);
            color: var(--text-color);
          }

        variables.css:
          :root {
            --bg-color: white;
            --text-color: black;
          }

        custom-resolver.js:
          const NodeResolver = require('@atlaspack/node-resolver-core').default;
          const { Resolver } = require('@atlaspack/plugin');

          // This custom resolver uses NodeResolver directly, forcing dependencies
          // through the JavaScript worker's runResolverResolve, which triggers
          // the BitFlags bug in the Dependency constructor
          module.exports = new Resolver({
            async loadConfig({ options }) {
              return new NodeResolver({
                fs: options.inputFS,
                projectRoot: options.projectRoot,
                packageManager: options.packageManager,
              });
            },

            async resolve({ dependency, specifier, config: resolver }) {
              return resolver.resolve({
                filename: specifier,
                specifierType: dependency.specifierType,
                parent: dependency.resolveFrom,
                env: dependency.env,
                sourcePath: dependency.sourcePath,
                packageConditions: dependency.packageConditions,
              });
            }
          });
    `;

    let b = await bundle(join(dir, 'index.js'), {
      inputFS,
    });

    // If we get here, the BitFlags conversion is working correctly
    assert.ok(b, 'Bundle should be created');
    let bundles = b.getBundles();
    assert.ok(bundles.length > 0, 'Should have at least one bundle');

    // Verify we have CSS bundles (proves CSS @imports with style condition were processed correctly)
    let cssBundles = bundles.filter((bundle) => bundle.type === 'css');
    assert.ok(
      cssBundles.length > 0,
      'Should have CSS bundle with @import resolved',
    );

    await inputFS.rimraf(dir);
  });

  describe('CSS minification', () => {
    it('minifies CSS output in production mode', async () => {
      await fsFixture(overlayFS, __dirname)`
        css-minify-prod
          index.css:
            body {
              color: red;
              background: white;
            }
            .heading {
              font-size: 2em;
              font-weight: bold;
            }
          yarn.lock:
      `;

      const bg = await bundle(join(__dirname, 'css-minify-prod/index.css'), {
        mode: 'production',
        inputFS: overlayFS,
        featureFlags: {fullNative: true},
      });

      const cssBundles = bg
        .getBundles()
        .filter((b: any) => b.type === 'css' && b.filePath);
      assert.ok(cssBundles.length > 0, 'Expected at least one CSS bundle');
      const css = await outputFS.readFile(cssBundles[0].filePath, 'utf8');

      // Minified output must not contain runs of whitespace between tokens.
      // A non-minified rule like `body {\n  color: red;\n}` becomes `body{color:red}`.
      assert.ok(
        !css.includes('  '),
        `Production CSS must not contain indentation whitespace; got: ${css}`,
      );
      assert.ok(
        !css.includes('{\n'),
        `Production CSS must not contain newlines after '{'; got: ${css}`,
      );
    });

    it('does not minify CSS output in development mode', async () => {
      await fsFixture(overlayFS, __dirname)`
        css-minify-dev
          index.css:
            body {
              color: red;
              background: white;
            }
            .heading {
              font-size: 2em;
              font-weight: bold;
            }
          yarn.lock:
      `;

      const bg = await bundle(join(__dirname, 'css-minify-dev/index.css'), {
        mode: 'development',
        inputFS: overlayFS,
        featureFlags: {fullNative: true},
      });

      const cssBundles = bg
        .getBundles()
        .filter((b: any) => b.type === 'css' && b.filePath);
      assert.ok(cssBundles.length > 0, 'Expected at least one CSS bundle');
      const css = await outputFS.readFile(cssBundles[0].filePath, 'utf8');

      // Development output must preserve whitespace — rules should be on
      // separate lines and properties should be indented.
      assert.ok(
        css.includes('\n'),
        `Development CSS must contain newlines (not minified); got: ${css}`,
      );
      assert.ok(
        css.includes('color'),
        `Development CSS must contain the 'color' property; got: ${css}`,
      );
    });
  });

  describe('worker thread logging', () => {
    // The v3 plugin host (AtlaspackWorker) runs inside a real worker_threads
    // Worker spawned by NapiWorkerPool. Log events are forwarded to the main
    // thread via parentPort.postMessage({type: 'logEvent', event}) and
    // re-emitted onto the main-thread @atlaspack/logger singleton by
    // NapiWorkerPool's message handler - unconditionally, regardless of
    // logLevel. These tests assert that end-to-end path works.
    //
    // Fixtures must be written to the real filesystem (inputFS) because the
    // v3 worker uses NodePackageManager which resolves against disk, not the
    // in-memory overlayFS.

    it('forwards plugin logger events from a worker thread to the main thread', async () => {
      const dir = join(__dirname, 'tmp', 'worker-logging');
      await inputFS.rimraf(dir);
      await inputFS.mkdirp(dir);

      await fsFixture(inputFS, dir)`
        index.js:
          export default 42;

        .parcelrc:
          {
            "extends": "@atlaspack/config-default",
            "transformers": {
              "*.js": ["./logging-transformer.js", "..."]
            },
            "reporters": ["@atlaspack/reporter-cli"]
          }

        logging-transformer.js:
          const {Transformer} = require('@atlaspack/plugin');
          module.exports = new Transformer({
            async transform({asset, logger}) {
              logger.warn({message: 'worker-thread-warn'});
              return [asset];
            }
          });

        yarn.lock:
      `;

      const stdio = mockStdio();

      try {
        await bundler(join(dir, 'index.js'), {
          inputFS,
          featureFlags: {atlaspackV3: true},
          defaultConfig: undefined, // Use .parcelrc in the fixture
          logLevel: 'warn',
        }).run();
      } finally {
        stdio.restore();
        await inputFS.rimraf(dir);
      }

      assert.ok(
        stdio.getStderr().includes('worker-thread-warn'),
        `Expected terminal output to contain "worker-thread-warn", but got:\nSTDOUT: ${stdio.getStdout()}\nSTDERR: ${stdio.getStderr()}`,
      );
    });

    it('forwards all log levels from a worker thread transformer', async () => {
      const dir = join(__dirname, 'tmp', 'worker-logging-levels');
      await inputFS.rimraf(dir);
      await inputFS.mkdirp(dir);

      await fsFixture(inputFS, dir)`
        index.js:
          export default 1;

        .parcelrc:
          {
            "extends": "@atlaspack/config-default",
            "transformers": {
              "*.js": ["./multi-level-transformer.js", "..."]
            },
            "reporters": ["@atlaspack/reporter-cli"]
          }

        multi-level-transformer.js:
          const {Transformer} = require('@atlaspack/plugin');
          module.exports = new Transformer({
            async transform({asset, logger}) {
              logger.verbose({message: 'verbose-msg'});
              logger.info({message: 'info-msg'});
              logger.warn({message: 'warn-msg'});
              logger.error({
                message: 'error-msg',
                codeFrames: [{
                  code: 'const x = 1;',
                  codeHighlights: [{start: {line: 1, column: 7}, end: {line: 1, column: 8}}]
                }]
              });
              return [asset];
            }
          });

        yarn.lock:
      `;

      const stdio = mockStdio();

      try {
        await bundler(join(dir, 'index.js'), {
          inputFS,
          featureFlags: {atlaspackV3: true},
          defaultConfig: undefined,
          logLevel: 'verbose',
        }).run();
      } finally {
        stdio.restore();
        await inputFS.rimraf(dir);
      }

      assert.ok(
        stdio.getStdout().includes('verbose-msg'),
        'verbose log not written to terminal',
      );
      assert.ok(
        stdio.getStdout().includes('info-msg'),
        'info log not written to terminal',
      );
      assert.ok(
        stdio.getStderr().includes('warn-msg'),
        'warn log not written to terminal',
      );
      assert.ok(
        stdio.getStderr().includes('error-msg'),
        'error log not written to terminal',
      );
      assert.ok(
        stdio.getStderr().includes('const x = 1;'),
        'codeframe not written to terminal',
      );
    });
  });
});
