// @flow
import assert from 'assert';
import invariant from 'assert';
import path from 'path';
import {
  bundle,
  bundler,
  describe,
  distDir,
  getNextBuild,
  inputFS as fs,
  it,
  outputFS,
  removeDistDirectory,
  run,
  sleep,
} from '@atlaspack/test-utils';
import Logger from '@atlaspack/logger';
import os from 'os';
import {spawnSync} from 'child_process';
import tempy from 'tempy';
import {md} from '@atlaspack/diagnostic';

const atlaspackCli = require.resolve('@atlaspack/cli/src/bin.js');
const inputDir = path.join(__dirname, '/input');

describe('babel', function () {
  let subscription;
  beforeEach(async function () {
    // TODO maybe don't do this for all tests
    await sleep(100);
    await outputFS.rimraf(inputDir);
    await sleep(100);
  });

  afterEach(async () => {
    await removeDistDirectory();
    if (subscription) {
      await subscription.unsubscribe();
      subscription = null;
    }
  });

  it.skip('auto installs @babel/core v7', async function () {
    let originalPkg = await fs.readFile(
      __dirname + '/integration/babel-7-autoinstall/package.json',
    );
    let b = await bundle(
      __dirname + '/integration/babel-7-autoinstall/index.js',
    );

    let output = await run(b);
    assert.equal(typeof output, 'object');
    assert.equal(typeof output.default, 'function');
    assert.equal(output.default(), 3);

    let pkg = await fs.readFile(
      __dirname + '/integration/babel-7-autoinstall/package.json',
    );
    assert(JSON.parse(pkg).devDependencies['@babel/core']);
    await fs.writeFile(
      __dirname + '/integration/babel-7-autoinstall/package.json',
      originalPkg,
    );
  });

  it.skip('auto installs babel plugins', async function () {
    let originalPkg = await fs.readFile(
      __dirname + '/integration/babel-plugin-autoinstall/package.json',
    );
    let b = await bundle(
      __dirname + '/integration/babel-plugin-autoinstall/index.js',
    );

    let output = await run(b);
    assert.equal(typeof output, 'object');
    assert.equal(typeof output.default, 'function');
    assert.equal(output.default(), 3);

    let pkg = await fs.readFile(
      __dirname + '/integration/babel-plugin-autoinstall/package.json',
    );
    assert(JSON.parse(pkg).devDependencies['@babel/core']);
    assert(
      JSON.parse(pkg).devDependencies[
        '@babel/plugin-proposal-class-properties'
      ],
    );
    await fs.writeFile(
      __dirname + '/integration/babel-plugin-autoinstall/package.json',
      originalPkg,
    );
  });

  it('compiles code using .babelrc config', async function () {
    await bundle(path.join(__dirname, '/integration/babelrc-custom/index.js'));

    let file = await outputFS.readFile(path.join(distDir, 'index.js'), 'utf8');
    assert(!file.includes('REPLACE_ME'));
    assert(file.includes('hello there'));
  });

  it('compiles code using babel.config.json config', async function () {
    let messages = [];
    let loggerDisposable = Logger.onLog((message) => {
      if (message.level !== 'verbose') {
        messages.push(message);
      }
    });
    await bundle(
      path.join(__dirname, '/integration/babel-config-json-custom/index.js'),
    );
    loggerDisposable.dispose();

    let file = await outputFS.readFile(path.join(distDir, 'index.js'), 'utf8');
    assert(!file.includes('REPLACE_ME'));
    assert(file.includes('hello there'));
    assert.deepEqual(messages, []);
  });

  it('compiles code using babel.config.js config', async function () {
    await bundle(
      path.join(__dirname, '/integration/babel-config-js/src/index.js'),
    );

    let file = await outputFS.readFile(path.join(distDir, 'index.js'), 'utf8');
    assert(!file.includes('REPLACE_ME'));
    assert(file.match(/return \d+;/));
  });

  it('compiles code using babel.config.js config that requires a plugin', async function () {
    await bundle(
      path.join(__dirname, '/integration/babel-config-js-require/src/index.js'),
    );

    let file = await outputFS.readFile(path.join(distDir, 'index.js'), 'utf8');
    assert(!file.includes('REPLACE_ME'));
    assert(file.match(/return \d+;/));
  });

  it('merges .babelrc and babel.config.json config in a monorepo', async function () {
    await bundle(
      path.join(
        __dirname,
        '/integration/babel-config-monorepo/packages/pkg-a/src/index.js',
      ),
    );

    let file = await outputFS.readFile(path.join(distDir, 'index.js'), 'utf8');
    assert(!file.includes('REPLACE_ME'));
    assert(file.includes('string from a plugin in babel.config.json'));
    assert(!file.includes('ANOTHER_THING_TO_REPLACE'));
    assert(file.includes('string from a plugin in .babelrc'));
    assert(file.includes('SOMETHING ELSE'));
    assert(!file.includes('string from a plugin from a different sub-package'));
  });

  it.skip('compiles code using browserslist', async function () {
    async function testBrowserListMultipleEnv(projectBasePath) {
      // Transpiled destructuring, like r = p.prop1, o = p.prop2, a = p.prop3;
      const prodRegExp =
        /\S+ ?= ?\S+\.prop1,\s*?\S+ ?= ?\S+\.prop2,\s*?\S+ ?= ?\S+\.prop3;/;
      // ES6 Destructuring, like in the source;
      const devRegExp =
        /const ?{\s*prop1(:.+)?,\s*prop2(:.+)?,\s*prop3(:.+)?\s*} ?= ?.*/;
      let file;
      // Dev build test
      await bundle(path.join(__dirname, projectBasePath, '/index.js'));
      file = await outputFS.readFile(path.join(distDir, 'index.js'), 'utf8');
      assert.equal(devRegExp.test(file), true);
      assert.equal(prodRegExp.test(file), false);
      // Prod build test
      await bundle(path.join(__dirname, projectBasePath, '/index.js'), {
        defaultTargetOptions: {
          shouldOptimize: false,
        },
      });
      file = await outputFS.readFile(path.join(distDir, 'index.js'), 'utf8');
      assert.equal(prodRegExp.test(file), true);
      assert.equal(devRegExp.test(file), false);
    }

    await testBrowserListMultipleEnv(
      '/integration/babel-browserslist-multiple-env',
    );
    await testBrowserListMultipleEnv(
      '/integration/babel-browserslist-multiple-env-as-string',
    );
  });

  it.skip('compiles node_modules code with browserslist to app target', async function () {
    await bundle(
      path.join(
        __dirname,
        '/integration/babel-node-modules-browserslist/index.js',
      ),
    );

    let file = await outputFS.readFile(path.join(distDir, 'index.js'), 'utf8');
    assert(file.includes('function Foo'));
    assert(file.includes('function Bar'));
  });

  it('supports multitarget builds using a custom babel config with @atlaspack/babel-preset-env', async function () {
    let fixtureDir = path.join(
      __dirname,
      '/integration/babel-config-js-multitarget',
    );

    await bundle(path.join(fixtureDir, 'src/index.js'));

    let [modern, legacy] = await Promise.all([
      outputFS.readFile(path.join(fixtureDir, 'dist/modern/index.js'), 'utf8'),
      outputFS.readFile(path.join(fixtureDir, 'dist/legacy/index.js'), 'utf8'),
    ]);

    assert(modern.includes('class Foo'));
    assert(modern.includes('this.x ** 2'));

    assert(!legacy.includes('class Foo'));
    assert(!legacy.includes('this.x ** 2'));

    await outputFS.rimraf(path.join(fixtureDir, 'dist'));
  });

  it.v2(
    'supports multitarget builds using a custom babel config with @atlaspack/babel-plugin-transform-runtime',
    async function () {
      let fixtureDir = path.join(
        __dirname,
        '/integration/babel-config-js-multitarget-transform-runtime',
      );

      await bundle(path.join(fixtureDir, 'src/index.js'), {
        mode: 'production',
        defaultTargetOptions: {
          shouldOptimize: false,
        },
      });

      let [main, esmodule] = await Promise.all([
        outputFS.readFile(path.join(fixtureDir, 'dist/main.js'), 'utf8'),
        outputFS.readFile(path.join(fixtureDir, 'dist/module.js'), 'utf8'),
      ]);

      assert(main.includes('"@babel/runtime/helpers/objectSpread2"'));
      assert(esmodule.includes('"@babel/runtime/helpers/esm/objectSpread2"'));

      await outputFS.rimraf(path.join(fixtureDir, 'dist'));
    },
  );

  it('compiles jsx', async function () {
    let b = await bundle(
      path.join(__dirname, '/integration/babel-jsx/index.jsx'),
    );

    let file = await outputFS.readFile(b.getBundles()[0].filePath, 'utf8');
    assert(file.includes('React.createElement'));
  });

  it('compiles ts', async function () {
    let b = await bundle(
      path.join(__dirname, '/integration/babel-ts/index.ts'),
    );

    let file = await outputFS.readFile(b.getBundles()[0].filePath, 'utf8');
    assert(!file.includes('interface'));
  });

  it('compiles tsx', async function () {
    let b = await bundle(
      path.join(__dirname, '/integration/babel-tsx/index.tsx'),
    );

    let file = await outputFS.readFile(b.getBundles()[0].filePath, 'utf8');
    assert(!file.includes('interface'));
    assert(file.includes('React.createElement'));
  });

  it('compiles code using a custom babel plugin and default transforms', async function () {
    let b = await bundle(
      path.join(__dirname, '/integration/babel-custom/index.js'),
    );

    let file = await outputFS.readFile(b.getBundles()[0].filePath, 'utf8');
    assert(!file.includes('REPLACE_ME'));

    let output = await run(b);
    assert.strictEqual(typeof output, 'object');
    assert.strictEqual(output.default, 'hello');
  });

  it('compiles code with shipped proposals when using @atlaspack/babel-preset-env', async function () {
    let b = await bundle(
      path.join(
        __dirname,
        '/integration/babel-preset-env-shippedProposals/index.js',
      ),
    );

    let file = await outputFS.readFile(b.getBundles()[0].filePath, 'utf8');
    assert(!file.includes('#priv'));

    let output = await run(b);
    assert.strictEqual(typeof output, 'object');
    assert.strictEqual(output.default, 123);
  });

  it.v2(
    'warns when a babel config contains only redundant plugins',
    async function () {
      let messages = [];
      let loggerDisposable = Logger.onLog((message) => {
        if (message.level !== 'verbose') {
          messages.push(message);
        }
      });
      let filePath = path.join(
        __dirname,
        '/integration/babel-warn-all/index.js',
      );
      await bundle(filePath);
      loggerDisposable.dispose();

      let babelrcPath = path.resolve(path.dirname(filePath), '.babelrc');
      assert.deepEqual(messages, [
        {
          type: 'log',
          level: 'warn',
          diagnostics: [
            {
              origin: '@atlaspack/transformer-babel',
              message: md`Parcel includes transpilation by default. Babel config __${path.relative(
                process.cwd(),
                babelrcPath,
              )}__ contains only redundant presets. Deleting it may significantly improve build performance.`,
              codeFrames: [
                {
                  filePath: babelrcPath,
                  codeHighlights: [
                    {
                      message: undefined,
                      start: {
                        line: 2,
                        column: 15,
                      },
                      end: {
                        line: 2,
                        column: 33,
                      },
                    },
                  ],
                },
              ],
              hints: [
                md`Delete __${path.relative(process.cwd(), babelrcPath)}__`,
              ],
              documentationURL:
                'https://parceljs.org/languages/javascript/#default-presets',
            },
            {
              origin: '@atlaspack/transformer-babel',
              message:
                "@babel/preset-env does not support Parcel's targets, which will likely result in unnecessary transpilation and larger bundle sizes.",
              codeFrames: [
                {
                  filePath: path.resolve(path.dirname(filePath), '.babelrc'),
                  codeHighlights: [
                    {
                      message: undefined,
                      start: {
                        line: 2,
                        column: 15,
                      },
                      end: {
                        line: 2,
                        column: 33,
                      },
                    },
                  ],
                },
              ],
              hints: [
                "Either remove __@babel/preset-env__ to use Parcel's builtin transpilation, or replace with __@atlaspack/babel-preset-env__",
              ],
              documentationURL:
                'https://parceljs.org/languages/javascript/#custom-plugins',
            },
          ],
        },
      ]);
    },
  );

  it.v2(
    'warns when a babel config contains redundant plugins',
    async function () {
      let messages = [];
      let loggerDisposable = Logger.onLog((message) => {
        if (message.level !== 'verbose') {
          messages.push(message);
        }
      });
      let filePath = path.join(
        __dirname,
        '/integration/babel-warn-some/index.js',
      );
      await bundle(filePath);
      loggerDisposable.dispose();

      let babelrcPath = path.resolve(path.dirname(filePath), '.babelrc');
      assert.deepEqual(messages, [
        {
          type: 'log',
          level: 'warn',
          diagnostics: [
            {
              origin: '@atlaspack/transformer-babel',
              message: md`Parcel includes transpilation by default. Babel config __${path.relative(
                process.cwd(),
                babelrcPath,
              )}__ includes the following redundant presets: __@atlaspack/babel-preset-env__. Removing these may improve build performance.`,
              codeFrames: [
                {
                  filePath: babelrcPath,
                  codeHighlights: [
                    {
                      message: undefined,
                      start: {
                        line: 2,
                        column: 15,
                      },
                      end: {
                        line: 2,
                        column: 43,
                      },
                    },
                  ],
                },
              ],
              hints: [
                md`Remove the above presets from __${path.relative(
                  process.cwd(),
                  babelrcPath,
                )}__`,
              ],
              documentationURL:
                'https://parceljs.org/languages/javascript/#default-presets',
            },
          ],
        },
      ]);
    },
  );

  it.v2(
    'warns when a JSON5 babel config contains redundant plugins',
    async function () {
      let messages = [];
      let loggerDisposable = Logger.onLog((message) => {
        if (message.level !== 'verbose') {
          messages.push(message);
        }
      });
      let filePath = path.join(
        __dirname,
        '/integration/babel-warn-some-json5/index.js',
      );
      await bundle(filePath);
      loggerDisposable.dispose();

      let babelrcPath = path.resolve(path.dirname(filePath), '.babelrc');
      assert.deepEqual(messages, [
        {
          type: 'log',
          level: 'warn',
          diagnostics: [
            {
              origin: '@atlaspack/transformer-babel',
              message: md`Parcel includes transpilation by default. Babel config __${path.relative(
                process.cwd(),
                babelrcPath,
              )}__ includes the following redundant presets: __@atlaspack/babel-preset-env__. Removing these may improve build performance.`,
              codeFrames: [
                {
                  filePath: babelrcPath,
                  codeHighlights: [
                    {
                      message: undefined,
                      start: {
                        line: 2,
                        column: 13,
                      },
                      end: {
                        line: 2,
                        column: 41,
                      },
                    },
                  ],
                },
              ],
              hints: [
                md`Remove the above presets from __${path.relative(
                  process.cwd(),
                  babelrcPath,
                )}__`,
              ],
              documentationURL:
                'https://parceljs.org/languages/javascript/#default-presets',
            },
          ],
        },
      ]);
    },
  );

  describe.v2('environment', () => {
    it('BABEL_ENV should be preferred to NODE_ENV', async () => {
      await bundle(
        path.join(__dirname, '/integration/babel-env-name/index.js'),
        {
          targets: {main: {distDir, engines: {browsers: ['ie 11']}}},
          env: {BABEL_ENV: 'production', NODE_ENV: 'development'},
        },
      );

      let file = await outputFS.readFile(
        path.join(distDir, 'index.js'),
        'utf8',
      );

      assert(!file.includes('class Foo'));
    });

    it('invalidates when BABEL_ENV changes', async () => {
      await bundle(
        path.join(__dirname, '/integration/babel-env-name/index.js'),
        {
          targets: {main: {distDir, engines: {}}},
          shouldDisableCache: false,
        },
      );
      let file = await outputFS.readFile(
        path.join(distDir, 'index.js'),
        'utf8',
      );
      assert(file.includes('class Foo'));

      await bundle(
        path.join(__dirname, '/integration/babel-env-name/index.js'),
        {shouldDisableCache: false, env: {BABEL_ENV: 'production'}},
      );
      file = await outputFS.readFile(path.join(distDir, 'index.js'), 'utf8');
      assert(!file.includes('class Foo'));
    });

    it('invalidates when NODE_ENV changes from BABEL_ENV', async () => {
      await bundle(
        path.join(__dirname, '/integration/babel-env-name/index.js'),
        {
          targets: {main: {distDir, engines: {}}},
          shouldDisableCache: false,
          env: {NODE_ENV: 'production'},
        },
      );
      let file = await outputFS.readFile(
        path.join(distDir, 'index.js'),
        'utf8',
      );
      assert(!file.includes('class Foo'));

      await bundle(
        path.join(__dirname, '/integration/babel-env-name/index.js'),
        {
          targets: {main: {distDir, engines: {}}},
          shouldDisableCache: false,
          env: {BABEL_ENV: 'development'},
        },
      );
      file = await outputFS.readFile(path.join(distDir, 'index.js'), 'utf8');
      assert(file.includes('class Foo'));
    });

    it('should be "production" if Atlaspack is run in production mode', async () => {
      await bundle(
        path.join(__dirname, '/integration/babel-env-name/index.js'),
        {
          targets: {main: {distDir, engines: {browsers: ['ie 11']}}},
          mode: 'production',
        },
      );
      let file = await outputFS.readFile(
        path.join(distDir, 'index.js'),
        'utf8',
      );
      assert(!file.includes('class Foo'));
    });
  });

  describe.skip('change detection', () => {
    afterEach(async () => {
      try {
        await fs.rimraf(inputDir);
        await fs.rimraf(distDir);
      } catch (e) {
        if (e.code === 'ENOENT') {
          throw e;
        }
      }
    });

    it('rebuilds when .babelrc changes', async function () {
      if (process.platform !== 'linux') {
        // This test is flaky outside of Linux. Skip it for now.
        return;
      }

      let inputDir = tempy.directory();
      let differentPath = path.join(inputDir, 'differentConfig');
      let configPath = path.join(inputDir, '.babelrc');

      await fs.ncp(
        path.join(__dirname, 'integration/babelrc-custom'),
        inputDir,
      );

      let b = bundler(path.join(inputDir, 'index.js'), {
        outputFS: fs,
        shouldAutoInstall: true,
      });

      subscription = await b.watch();
      await getNextBuild(b);
      let distFile = await fs.readFile(path.join(distDir, 'index.js'), 'utf8');
      assert(distFile.includes('hello there'));
      await fs.copyFile(differentPath, configPath);
      await new Promise((resolve) => setTimeout(resolve, 100));
      // On Windows only, `fs.utimes` arguments must be instances of `Date`,
      // otherwise it fails. For Mac instances on Azure CI, using a Date instance
      // does not update the utime correctly, so for all other platforms, use a
      // number.
      // https://github.com/nodejs/node/issues/5561
      let now = os.platform() === 'win32' ? new Date() : Date.now();
      // fs.copyFile does not reliably update mtime, which babel uses to invalidate cached file contents
      await fs.utimes(configPath, now, now);
      await getNextBuild(b);
      distFile = await fs.readFile(path.join(distDir, 'index.js'), 'utf8');
      assert(!distFile.includes('hello there'));
      assert(distFile.includes('something different'));
    });

    it.skip('rebuilds when declared external dependencies change', async function () {
      let inputDir = tempy.directory();
      let filepathMain = path.join(inputDir, 'main.txt');
      let filepathFallback = path.join(inputDir, 'fallback.txt');

      await fs.ncp(
        path.join(__dirname, 'integration/babel-external-deps'),
        inputDir,
      );

      let b = bundler(path.join(inputDir, 'index.js'), {
        outputFS: fs,
        shouldAutoInstall: true,
      });

      subscription = await b.watch();

      async function step(f, positive, negative) {
        if (f != null) {
          await fs.writeFile(f, positive);
        }
        let build = await getNextBuild(b);
        invariant(build.type === 'buildSuccess');
        let distFile = await fs.readFile(
          build.bundleGraph.getBundles()[0].filePath,
          'utf8',
        );
        assert(distFile.includes(positive));
        if (negative != null) {
          assert(!distFile.includes(negative));
        }
      }

      await step(null, 'foo1', null);
      await step(filepathFallback, 'foo2', 'foo1');
      await step(filepathMain, 'foo3', 'foo2');
      await step(filepathMain, 'foo4', 'foo3');
    });

    it('invalidates babel.config.js across runs', async function () {
      let dateRe = /return (\d+);/;

      let fixtureDir = path.join(__dirname, '/integration/babel-config-js');
      let distDir = path.resolve(fixtureDir, './dist');
      let cacheDir = path.resolve(fixtureDir, '.parcel-cache');
      await fs.rimraf(distDir);
      await fs.rimraf(cacheDir);
      await fs.rimraf(path.resolve(fixtureDir, './node_modules/.cache'));

      let build = () =>
        spawnSync(
          'node',
          [
            atlaspackCli,
            'build',
            'src/index.js',
            '--no-optimize',
            '--no-scope-hoist',
          ],
          {
            cwd: fixtureDir,
            env: {
              ...process.env,
              ATLASPACK_WORKERS: '0',
            },
          },
        );

      build();
      let file = await fs.readFile(path.join(distDir, 'index.js'), 'utf8');
      assert(!file.includes('REPLACE_ME'));
      let firstMatch = file.match(dateRe);
      assert(firstMatch != null);
      let firstDatestamp = firstMatch[1];

      build();
      file = await fs.readFile(path.join(distDir, 'index.js'), 'utf8');
      let secondMatch = file.match(dateRe);
      assert(secondMatch != null);
      let secondDatestamp = secondMatch[1];

      assert.notEqual(firstDatestamp, secondDatestamp);
    });

    it('invalidates when babel plugins are upgraded across runs', async function () {
      let fixtureDir = path.join(
        __dirname,
        '/integration/babel-plugin-upgrade',
      );
      await fs.ncp(path.join(fixtureDir), inputDir);
      await fs.rimraf(path.join(__dirname, '.parcel-cache'));

      let build = () =>
        spawnSync(
          'node',
          [
            atlaspackCli,
            'build',
            'index.js',
            '--no-optimize',
            '--no-scope-hoist',
          ],
          {
            cwd: inputDir,
            env: {
              ...process.env,
              ATLASPACK_WORKERS: '0',
            },
          },
        );

      build();
      let file = await fs.readFile(
        path.join(inputDir, 'dist', 'index.js'),
        'utf8',
      );
      assert(!file.includes('REPLACE_ME'));
      assert(file.includes('hello there'));

      await fs.writeFile(
        path.join(inputDir, 'node_modules/babel-plugin-dummy/message.js'),
        'module.exports = "something different"',
      );
      await fs.writeFile(
        path.join(inputDir, 'node_modules/babel-plugin-dummy/package.json'),
        JSON.stringify({name: 'babel-plugin-dummy', version: '1.1.0'}),
      );
      await fs.writeFile(
        path.join(inputDir, 'yarn.lock'),
        '# yarn.lock has been updated',
      );

      build();
      file = await fs.readFile(path.join(inputDir, 'dist', 'index.js'), 'utf8');
      assert(!file.includes('REPLACE_ME'));
      assert(!file.includes('hello there'));
      assert(file.includes('something different'));
    });
  });
});
