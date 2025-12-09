import assert from 'assert';
import path from 'path';
import {
  bundle,
  describe,
  distDir,
  inputFS as fs,
  it,
  outputFS,
  overlayFS,
  run,
  ncp,
  fsFixture,
  isAtlaspackV3,
} from '@atlaspack/test-utils';
import {symlinkSync} from 'fs';
import nullthrows from 'nullthrows';

const inputDir = path.join(__dirname, '/input');

describe('transpilation', function () {
  let featureFlags = {
    newJsxConfig: isAtlaspackV3,
  };

  it('should not transpile if no targets are defined', async function () {
    await bundle(path.join(__dirname, '/integration/babel-default/index.js'), {
      defaultTargetOptions: {
        engines: undefined,
        shouldOptimize: false,
      },
      featureFlags,
    });
    let file = await outputFS.readFile(path.join(distDir, 'index.js'), 'utf8');
    assert(file.includes('class Foo'));
    assert(file.includes('class Bar'));
  });

  it('should support transpiling using browserlist', async function () {
    await bundle(
      path.join(__dirname, '/integration/babel-browserslist/index.js'),
      {
        featureFlags,
      },
    );

    let file = await outputFS.readFile(path.join(distDir, 'index.js'), 'utf8');
    assert(file.includes('function Foo'));
    assert(file.includes('function Bar'));
  });

  it('should support transpiling when engines have semver ranges', async () => {
    let fixtureDir = path.join(__dirname, '/integration/babel-semver-engine');
    await bundle(path.join(fixtureDir, 'index.js'), {featureFlags});

    let legacy = await outputFS.readFile(
      path.join(fixtureDir, 'dist', 'legacy.js'),
      'utf8',
    );
    assert(legacy.includes('function Foo'));
    assert(legacy.includes('function Bar'));

    let modern = await outputFS.readFile(
      path.join(fixtureDir, 'dist', 'modern.js'),
      'utf8',
    );
    assert(modern.includes('class Foo'));
    assert(modern.includes('class Bar'));
  });

  it('should transpile node_modules by default', async function () {
    let b = await bundle(
      path.join(__dirname, '/integration/babel-node-modules/index.js'),
      {featureFlags},
    );

    let file = await outputFS.readFile(path.join(distDir, 'index.js'), 'utf8');
    assert(!/class \S+ \{/.test(file));
    assert(file.includes('function Bar'));
    let res = await run(b);
    assert.equal(res.t, 'function');
  });

  it('should not support JSX in node_modules', async function () {
    await assert.rejects(() =>
      bundle(
        path.join(__dirname, '/integration/babel-node-modules-jsx/index.js'),
        {featureFlags},
      ),
    );
  });

  it('should compile node_modules with a source field in package.json when not symlinked', async function () {
    await bundle(
      path.join(
        __dirname,
        '/integration/babel-node-modules-source-unlinked/index.js',
      ),
      {featureFlags},
    );

    let file = await outputFS.readFile(path.join(distDir, 'index.js'), 'utf8');
    assert(file.includes('function Foo'));
    assert(file.includes('function Bar'));
  });

  it('should support compiling JSX', async function () {
    await bundle(path.join(__dirname, '/integration/jsx/index.jsx'), {
      featureFlags,
    });

    let file = await outputFS.readFile(path.join(distDir, 'index.js'), 'utf8');
    assert(file.includes('React.createElement("div"'));
    assert(file.includes('fileName: "integration/jsx/index.jsx"'));
  });

  describe('supports compiling JSX', () => {
    it('with member expression type', async function () {
      await bundle(path.join(__dirname, '/integration/jsx-member/index.jsx'), {
        featureFlags,
      });

      let file = await outputFS.readFile(
        path.join(distDir, 'index.js'),
        'utf8',
      );
      assert(file.includes('React.createElement(S.Foo'));
    });

    it('with pure annotations', async function () {
      let b = await bundle(
        path.join(__dirname, '/integration/jsx-react/pure-comment.js'),
        {featureFlags},
      );

      let file = await outputFS.readFile(
        path.join(distDir, 'pure-comment.js'),
        'utf8',
      );
      assert(
        file.includes('/*#__PURE__*/ (0, _reactDefault.default).createElement'),
      );

      let res = await run(b);
      assert(res.Foo());
    });

    it('spread with modern targets', async function () {
      let b = await bundle(
        path.join(__dirname, '/integration/jsx-spread/index.jsx'),
        {featureFlags},
      );

      let file = await outputFS.readFile(b.getBundles()[0].filePath, 'utf8');
      assert(file.includes('React.createElement("div"'));
      assert(file.includes('...a'));
      assert(!file.includes('@swc/helpers'));
    });

    it('in js files with a React dependency', async function () {
      await bundle(path.join(__dirname, '/integration/jsx-react/index.js'), {
        featureFlags,
      });

      let file = await outputFS.readFile(
        path.join(distDir, 'index.js'),
        'utf8',
      );
      assert(file.includes('React.createElement("div"'));
    });

    // JSX Configuration Tests
    // These integration tests ensure that the JSX parsing and transformation logic for:
    // - v2 -> packages/transformers/js/src/JSTransformer.ts
    // - v3 -> crates/atlaspack_plugin_transformer_js/src/js_transformer.rs
    // are consistent.
    describe('JSX parsing and transformation', () => {
      let dir: string = path.join(__dirname, 'jsx-configuration-fixture');

      beforeEach(async function () {
        await overlayFS.rimraf(dir);
        await overlayFS.mkdirp(dir);
      });

      it('transforms JSX in .js files with React dependency and tsconfig', async function () {
        await fsFixture(overlayFS, dir)`
          package.json:
            {
              "private": true,
              "dependencies": {
                "react": "*"
              }
            }

          tsconfig.json:
            {
              "compilerOptions": {
                "jsx": "react",
                "target": "es2015"
              }
            }

          index.js:
            module.exports = <div>"First we bundle, then we ball." - Sun Tzu, The Art of War</div>;
        `;

        await bundle(path.join(dir, 'index.js'), {
          inputFS: overlayFS,
          featureFlags,
        });

        let file = await outputFS.readFile(
          path.join(distDir, 'index.js'),
          'utf8',
        );
        // JSX should be transformed
        assert(!file.includes('<div'), 'JSX should be transformed');
        // JSX should be transformed with React pragmas
        assert(file.includes('React.createElement("div"'));
      });

      it('fails to parse JSX in .js files without React dependency or tsconfig', async function () {
        await fsFixture(overlayFS, dir)`
          package.json:
            {
              "private": true,
              "dependencies": {}
            }

          index.js:
            module.exports = <div>"First we bundle, then we ball." - Sun Tzu, The Art of War</div>;
        `;

        await assert.rejects(
          () =>
            bundle(path.join(dir, 'index.js'), {
              inputFS: overlayFS,
            }),
          'JSX parsing should fail when no React dependency and no tsconfig',
        );
      });

      it('transforms JSX in .js files with React dependency but no tsconfig', async function () {
        await fsFixture(overlayFS, dir)`
          package.json:
            {
              "private": true,
              "dependencies": {
                "react": "*"
              }
            }

          index.js:
            module.exports = <div>"First we bundle, then we ball." - Sun Tzu, The Art of War</div>;
        `;

        await bundle(path.join(dir, 'index.js'), {
          inputFS: overlayFS,
          featureFlags,
        });

        let file = await outputFS.readFile(
          path.join(distDir, 'index.js'),
          'utf8',
        );
        // JSX should be transformed
        assert(!file.includes('<div'), 'JSX should be transformed');
        // JSX should be transformed with React pragmas
        assert(file.includes('React.createElement("div"'));
      });

      it('transforms JSX in .jsx files without React dependency or tsconfig', async function () {
        await fsFixture(overlayFS, dir)`
          package.json:
            {
              "private": true,
              "dependencies": {}
            }

          index.jsx:
            module.exports = <div>"First we bundle, then we ball." - Sun Tzu, The Art of War</div>;
        `;

        await bundle(path.join(dir, 'index.jsx'), {
          inputFS: overlayFS,
          featureFlags,
        });

        // Output file is normalized to .js despite input file .jsx extension
        let file = await outputFS.readFile(
          path.join(distDir, 'index.js'),
          'utf8',
        );
        // JSX should be transformed (file extension enables JSX)
        assert(!file.includes('<div'), 'JSX should be transformed');
        // JSX should be transformed with React pragmas
        assert(file.includes('React.createElement("div"'));
      });

      it('transforms JSX in .jsx files with React dependency but no tsconfig', async function () {
        await fsFixture(overlayFS, dir)`
          package.json:
            {
              "private": true,
              "dependencies": {
                "react": "*"
              }
            }

          index.jsx:
            module.exports = <div>"First we bundle, then we ball." - Sun Tzu, The Art of War</div>;
        `;

        await bundle(path.join(dir, 'index.jsx'), {
          inputFS: overlayFS,
          featureFlags,
        });

        // Output file is normalized to .js despite input file .jsx extension
        let file = await outputFS.readFile(
          path.join(distDir, 'index.js'),
          'utf8',
        );
        // JSX should be transformed
        assert(!file.includes('<div'), 'JSX should be transformed');
        // JSX should be transformed with React pragmas
        assert(file.includes('React.createElement("div"'));
      });

      it('fails to parse JSX in .ts files even with React dependency and tsconfig', async function () {
        await fsFixture(overlayFS, dir)`
          package.json:
            {
              "private": true,
              "dependencies": {
                "react": "*"
              }
            }

          tsconfig.json:
            {
              "compilerOptions": {
                "jsx": "react",
                "target": "es2015"
              }
            }

          index.ts:
            module.exports = <div>"First we bundle, then we ball." - Sun Tzu, The Art of War</div>;
        `;

        await assert.rejects(
          () =>
            bundle(path.join(dir, 'index.ts'), {
              inputFS: overlayFS,
            }),
          'JSX parsing should fail in .ts files even with React dependency and tsconfig',
        );
      });

      it('transforms JSX in .tsx files with React dependency but no tsconfig', async function () {
        await fsFixture(overlayFS, dir)`
          package.json:
            {
              "private": true,
              "dependencies": {
                "react": "*"
              }
            }

          index.tsx:
            module.exports = <div>"First we bundle, then we ball." - Sun Tzu, The Art of War</div>;
        `;

        await bundle(path.join(dir, 'index.tsx'), {
          inputFS: overlayFS,
          featureFlags,
        });

        // Output file is normalized to .js despite input file .tsx extension
        let file = await outputFS.readFile(
          path.join(distDir, 'index.js'),
          'utf8',
        );
        // JSX should be transformed
        assert(!file.includes('<div'), 'JSX should be transformed');
        // JSX should be transformed with React pragmas
        assert(file.includes('React.createElement("div"'));
      });
    });

    // Non react frameworks are not currently supported in v3.
    it.v2('in js files with React aliased to Preact', async function () {
      await bundle(
        path.join(__dirname, '/integration/jsx-react-alias/index.js'),
      );

      let file = await outputFS.readFile(
        path.join(distDir, 'index.js'),
        'utf8',
      );
      assert(file.includes('React.createElement("div"'));
    });

    // Non react frameworks are not currently supported in v3.
    it.v2('in js files with Preact dependency', async function () {
      await bundle(path.join(__dirname, '/integration/jsx-preact/index.js'));

      let file = await outputFS.readFile(
        path.join(distDir, 'index.js'),
        'utf8',
      );
      assert(file.includes('h("div"'));
    });

    // Non react frameworks are not currently supported in v3.
    it.v2('in ts files with Preact dependency', async function () {
      let b = await bundle(
        path.join(__dirname, '/integration/jsx-preact-ts/index.tsx'),
      );

      assert(typeof (await run(b)) === 'object');
    });

    // Non react frameworks are not currently supported in v3.
    it.v2('in js files with Preact url dependency', async function () {
      await bundle(
        path.join(__dirname, '/integration/jsx-preact-with-url/index.js'),
      );

      let file = await outputFS.readFile(
        path.join(distDir, 'index.js'),
        'utf8',
      );
      assert(file.includes('h("div"'));
    });

    // Non react frameworks are not currently supported in v3.
    it.v2('in js files with Nerv dependency', async function () {
      await bundle(path.join(__dirname, '/integration/jsx-nervjs/index.js'));

      let file = await outputFS.readFile(
        path.join(distDir, 'index.js'),
        'utf8',
      );
      assert(file.includes('Nerv.createElement("div"'));
    });

    // Non react frameworks are not currently supported in v3.
    it.v2('in js files with Hyperapp dependency', async function () {
      await bundle(path.join(__dirname, '/integration/jsx-hyperapp/index.js'));

      let file = await outputFS.readFile(
        path.join(distDir, 'index.js'),
        'utf8',
      );
      assert(file.includes('h("div"'));
    });
  });

  describe.v2('supports the automatic jsx runtime', () => {
    it('with React >= 17', async function () {
      let b = await bundle(
        path.join(__dirname, '/integration/jsx-automatic/index.js'),
      );

      let file = await outputFS.readFile(b.getBundles()[0].filePath, 'utf8');
      assert(file.includes('react/jsx-dev-runtime'));
      assert(file.includes('(0, _jsxDevRuntime.jsxDEV)("div"'));
    });

    it('with Preact >= 10.5', async function () {
      let b = await bundle(
        path.join(__dirname, '/integration/jsx-automatic-preact/index.js'),
      );

      let file = await outputFS.readFile(b.getBundles()[0].filePath, 'utf8');
      assert(file.includes('preact/jsx-dev-runtime'));
      assert(file.includes('(0, _jsxDevRuntime.jsxDEV)("div"'));
    });

    it('with React ^16.14.0', async function () {
      let b = await bundle(
        path.join(__dirname, '/integration/jsx-automatic-16/index.js'),
      );

      let file = await outputFS.readFile(b.getBundles()[0].filePath, 'utf8');
      assert(file.includes('react/jsx-dev-runtime'));
      assert(file.includes('(0, _jsxDevRuntime.jsxDEV)("div"'));
    });

    it('with React 18 prereleases', async function () {
      let b = await bundle(
        path.join(__dirname, '/integration/jsx-automatic-18/index.js'),
      );

      let file = await outputFS.readFile(b.getBundles()[0].filePath, 'utf8');
      assert(file.includes('react/jsx-dev-runtime'));
      assert(file.includes('(0, _jsxDevRuntime.jsxDEV)("div"'));
    });

    it('with experimental React versions', async function () {
      let b = await bundle(
        path.join(
          __dirname,
          '/integration/jsx-automatic-experimental/index.js',
        ),
      );

      let file = await outputFS.readFile(b.getBundles()[0].filePath, 'utf8');
      assert(file.includes('react/jsx-dev-runtime'));
      assert(file.includes('(0, _jsxDevRuntime.jsxDEV)("div"'));
    });

    it('with Preact alias', async function () {
      let b = await bundle(
        path.join(
          __dirname,
          '/integration/jsx-automatic-preact-with-alias/index.js',
        ),
      );

      let file = await outputFS.readFile(b.getBundles()[0].filePath, 'utf8');
      assert(/\Wreact\/jsx-dev-runtime\W/.test(file));
      assert(file.includes('(0, _jsxDevRuntime.jsxDEV)("div"'));
    });

    it('with explicit tsconfig.json', async function () {
      let b = await bundle(
        path.join(__dirname, '/integration/jsx-automatic-tsconfig/index.js'),
      );

      let file = await outputFS.readFile(b.getBundles()[0].filePath, 'utf8');
      assert(file.includes('preact/jsx-dev-runtime'));
      assert(file.includes('(0, _jsxDevRuntime.jsxDEV)("div"'));
    });
  });

  describe('of tsconfig.json', () => {
    it.v2('supports explicit JSX pragma', async function () {
      let b = await bundle(
        path.join(__dirname, '/integration/jsx-pragma-tsconfig/index.js'),
      );

      let file = await outputFS.readFile(b.getBundles()[0].filePath, 'utf8');
      assert(file.includes('JSX(JSXFragment'));
      assert(file.includes('JSX("div"'));
    });

    it.v2('supports explicitly enabling JSX', async function () {
      let b = await bundle(
        path.join(__dirname, '/integration/jsx-tsconfig/index.js'),
      );

      let file = await outputFS.readFile(b.getBundles()[0].filePath, 'utf8');
      assert(file.includes('React.createElement("div"'));
    });

    it('supports decorators', async function () {
      let b = await bundle(
        path.join(__dirname, '/integration/decorators/index.ts'),
        {featureFlags},
      );

      let output: Array<any> = [];
      await run(b, {
        output(o: any) {
          output.push(o);
        },
      });

      assert.deepEqual(output, [
        'first(): factory evaluated',
        'second(): factory evaluated',
        'second(): called',
        'first(): called',
      ]);
    });

    it('supports decorators and setting useDefineForClassFields', async function () {
      let b = await bundle(
        path.join(
          __dirname,
          '/integration/decorators-useDefineForClassFields/index.ts',
        ),
        {featureFlags},
      );

      let output: Array<any> = [];
      await run(b, {
        output(...o) {
          output.push(...o);
        },
      });

      assert.deepEqual(output, ['foo 15', 'foo 16']);
    });
  });

  it('should support transpiling optional chaining', async function () {
    let b = await bundle(
      path.join(__dirname, '/integration/babel-optional-chaining/index.js'),
      {featureFlags},
    );

    let file = await outputFS.readFile(b.getBundles()[0].filePath, 'utf8');
    assert(!file.includes('?.'));

    let output = await run(b);
    assert.equal(typeof output, 'object');
    assert.deepEqual(output.default, [undefined, undefined]);
  });

  it('should only include necessary parts of core-js using browserlist', async function () {
    await bundle(path.join(__dirname, '/integration/babel-core-js/index.js'), {
      featureFlags,
    });

    let file = await outputFS.readFile(path.join(distDir, 'index.js'), 'utf8');
    // console.log(file)
    assert(file.includes('async function Bar() {'));
    // Check that core-js's globalThis polyfill is referenced.
    // NOTE: This may change if core-js internals change.
    assert(file.includes('esnext.global-this'));
    assert(!file.includes('es.array.concat'));
  });

  it.v2(
    'should resolve @swc/helpers and regenerator-runtime relative to parcel',
    async function () {
      let dir = path.join('/tmp/' + Math.random().toString(36).slice(2));
      await outputFS.mkdirp(dir);
      await ncp(path.join(__dirname, '/integration/swc-helpers'), dir);
      await bundle(path.join(dir, 'index.js'), {
        mode: 'production',
        inputFS: overlayFS,
        defaultTargetOptions: {
          engines: {
            browsers: '>= 0.25%',
          },
        },
      });
    },
  );

  it.v2(
    'should support commonjs and esm versions of @swc/helpers',
    async function () {
      let b = await bundle(
        path.join(__dirname, '/integration/swc-helpers-library/index.js'),
      );

      let file = await outputFS.readFile(
        nullthrows(
          b.getBundles().find((b) => b.env.outputFormat === 'commonjs'),
        ).filePath,
        'utf8',
      );
      assert(file.includes('@swc/helpers/cjs/_class_call_check.cjs'));

      file = await outputFS.readFile(
        nullthrows(
          b.getBundles().find((b) => b.env.outputFormat === 'esmodule'),
        ).filePath,
        'utf8',
      );
      assert(file.includes('@swc/helpers/_/_class_call_check'));
    },
  );

  it.v2(
    'should support commonjs versions of @swc/helpers without scope hoisting',
    async function () {
      let b = await bundle(
        path.join(__dirname, '/integration/swc-helpers-library/index.js'),
        {
          targets: {
            test: {
              distDir,
              isLibrary: true,
              scopeHoist: false,
            },
          },
        },
      );

      let file = await outputFS.readFile(b.getBundles()[0].filePath, 'utf8');
      assert(file.includes('@swc/helpers/cjs/_class_call_check.cjs'));
      await run(b);
    },
  );

  it.v2('should print errors from transpilation', async function () {
    let source = path.join(
      __dirname,
      '/integration/transpilation-invalid/index.js',
    );
    await assert.rejects(() => bundle(source), {
      name: 'BuildError',
      diagnostics: [
        {
          codeFrames: [
            {
              codeHighlights: [
                {
                  message: undefined,
                  start: {
                    column: 1,
                    line: 1,
                  },
                  end: {
                    column: 12,
                    line: 1,
                  },
                },
              ],
              filePath: source,
            },
          ],
          hints: null,
          message: 'pragma cannot be set when runtime is automatic',
          origin: '@atlaspack/transformer-js',
        },
        {
          codeFrames: [
            {
              codeHighlights: [
                {
                  message: undefined,
                  start: {
                    column: 3,
                    line: 9,
                  },
                  end: {
                    column: 4,
                    line: 9,
                  },
                },
              ],
              filePath: source,
            },
          ],
          hints: null,
          message: 'duplicate private name #x.',
          origin: '@atlaspack/transformer-js',
        },
      ],
    });
  });

  describe('tests needing the real filesystem', () => {
    afterEach(async () => {
      if (process.platform === 'win32') {
        return;
      }

      try {
        await fs.rimraf(inputDir);
        await fs.rimraf(distDir);
      } catch (e: any) {
        // ignore
      }
    });

    it('should compile node_modules when symlinked with a source field in package.json', async function () {
      if (process.platform === 'win32') {
        this.skip();
        return;
      }

      const inputDir = path.join(__dirname, '/input');
      await fs.rimraf(inputDir);
      await fs.mkdirp(path.join(inputDir, 'node_modules'));
      await fs.ncp(
        path.join(
          path.join(__dirname, '/integration/babel-node-modules-source'),
        ),
        inputDir,
      );

      // Create the symlink here to prevent cross platform and git issues
      symlinkSync(
        path.join(inputDir, 'packages/foo'),
        path.join(inputDir, 'node_modules/foo'),
        'dir',
      );

      await bundle(inputDir + '/index.js', {outputFS: fs, featureFlags});

      let file = await fs.readFile(path.join(distDir, 'index.js'), 'utf8');
      assert(file.includes('function Foo'));
      assert(file.includes('function Bar'));
    });
  });

  describe('JSX configuration with newJsxConfig feature flag', () => {
    let dir: string = path.join(__dirname, 'jsx-new-config-fixture');

    beforeEach(async function () {
      await overlayFS.rimraf(dir);
      await overlayFS.mkdirp(dir);
    });

    it('should use custom JSX pragma from package.json config', async () => {
      await fsFixture(overlayFS, dir)`
        package.json:
          {
            "name": "jsx-pragma-test",
            "@atlaspack/transformer-js": {
              "jsx": {
                "pragma": "customPragma",
                "pragmaFragment": "Fragment"
              }
            }
          }

        yarn.lock:

        index.js:
          const Component = () => <div>Custom JSX works!</div>;
          module.exports = Component;
      `;

      let b = await bundle(path.join(dir, 'index.js'), {
        inputFS: overlayFS,
        featureFlags: {
          newJsxConfig: true,
        },
      });

      // Just verify the bundle compiles successfully and uses custom pragma
      let js = await outputFS.readFile(b.getBundles()[0].filePath, 'utf8');
      assert(
        js.includes('customPragma'),
        'Should use customPragma instead of React.createElement',
      );
      assert(
        !js.includes('React.createElement'),
        'Should not contain React.createElement',
      );
    });

    it('should enable automatic JSX runtime for all files', async () => {
      await fsFixture(overlayFS, dir)`
        package.json:
          {
            "name": "jsx-automatic-runtime-test",
            "@atlaspack/transformer-js": {
              "jsx": {
                "importSource": "react",
                "automaticRuntime": {"Enabled": true}
              }
            }
          }

        yarn.lock:

        index.jsx:
          const Component = () => <div>Automatic runtime works!</div>;
          module.exports = Component;
      `;

      let b = await bundle(path.join(dir, 'index.jsx'), {
        inputFS: overlayFS,
        featureFlags: {
          newJsxConfig: true,
        },
      });

      // Verify automatic runtime imports and jsx usage
      let js = await outputFS.readFile(b.getBundles()[0].filePath, 'utf8');
      assert(
        js.includes('jsx') || js.includes('jsxDEV'),
        'Should use jsx() or jsxDEV() function from automatic runtime',
      );
      assert(
        js.includes('react/jsx-runtime') ||
          js.includes('react/jsx-dev-runtime') ||
          js.includes('jsx-runtime'),
        'Should import from jsx-runtime or jsx-dev-runtime',
      );
      // Note: Automatic runtime may still contain some React.createElement for certain elements
    });

    it('should enable automatic JSX runtime for matching glob patterns only', async () => {
      await fsFixture(overlayFS, dir)`
        package.json:
          {
            "name": "jsx-glob-runtime-test",
            "@atlaspack/transformer-js": {
              "jsx": {
                "pragma": "React.createElement",
                "pragmaFragment": "React.Fragment",
                "importSource": "react",
                "automaticRuntime": {"Glob": ["modern/**/*.jsx"]}
              }
            }
          }

        yarn.lock:

        index.js:
          // Mock React and jsx runtime
          const React = {
            createElement: (type, props, ...children) => {
              if (type === 'span') return children[0];
              return { type, props, children };
            }
          };

          const jsx = (type, props) => {
            if (type === 'span') return props.children;
            return { type, props };
          };

          global.React = React;
          global.require = global.require || function(id) {
            if (id === 'react/jsx-runtime') {
              return { jsx };
            }
            return {};
          };

          const ModernComponent = require('./modern/Component.jsx');
          const LegacyComponent = require('./legacy/Component.jsx');

          module.exports = 'Glob runtime works!';

        modern/Component.jsx:
          const ModernComponent = () => <span>modern</span>;
          module.exports = ModernComponent;

        legacy/Component.jsx:
          const LegacyComponent = () => <span>legacy</span>;
          module.exports = LegacyComponent;
      `;

      let b = await bundle(path.join(dir, 'index.js'), {
        inputFS: overlayFS,
        featureFlags: {
          newJsxConfig: true,
        },
      });

      let output = await run(b);
      assert.equal(output, 'Glob runtime works!');

      // Check that different runtime transformations are applied
      let js = await outputFS.readFile(b.getBundles()[0].filePath, 'utf8');

      // Modern component should use automatic runtime
      assert(
        js.includes('jsx(') || js.includes('jsx-runtime'),
        'Modern component should use automatic jsx runtime',
      );

      // Legacy component should use classic runtime
      assert(
        js.includes('React.createElement'),
        'Legacy component should use React.createElement',
      );
    });

    it('should fall back to default React pragmas without explicit config', async () => {
      await fsFixture(overlayFS, dir)`
        package.json:
          {
            "name": "jsx-default-test"
          }

        yarn.lock:

        component.jsx:
          // Mock React for default pragma testing
          const React = {
            createElement: (type, props, ...children) => {
              if (type === 'div') return children[0];
              return { type, props, children };
            },
            Fragment: (props, ...children) => children
          };

          global.React = React;

          const Component = () => <div>Default JSX works!</div>;
          module.exports = Component();
      `;

      let b = await bundle(path.join(dir, 'component.jsx'), {
        inputFS: overlayFS,
        featureFlags: {
          newJsxConfig: true,
        },
      });

      let output = await run(b);
      assert.equal(output, 'Default JSX works!');

      // Verify default React pragmas are used
      let js = await outputFS.readFile(b.getBundles()[0].filePath, 'utf8');
      assert(
        js.includes('React.createElement'),
        'Should use React.createElement by default',
      );
      assert(
        !js.includes('jsx('),
        'Should not use automatic jsx runtime without explicit config',
      );
    });

    it('should not enable JSX for .js files without explicit config', async () => {
      await fsFixture(overlayFS, dir)`
        package.json:
          {
            "name": "jsx-js-no-config-test"
          }

        yarn.lock:

        index.js:
          // This .js file should NOT transform JSX without explicit config
          // In the new behavior, we should get a build error or JSX should remain untransformed
          module.exports = "No JSX transformation applied";

        index-with-jsx.js:
          // This should fail to build because JSX isn't enabled for .js files without explicit config
          const element = <div>This should not work</div>;
          module.exports = element;
      `;

      // Test that normal JS works without JSX config
      let b = await bundle(path.join(dir, 'index.js'), {
        inputFS: overlayFS,
        featureFlags: {
          newJsxConfig: true,
        },
      });

      let output = await run(b);
      assert.equal(output, 'No JSX transformation applied');

      // Test behavior of JSX in .js files without explicit config (differs between V2 and V3)
      if (process.env.ATLASPACK_V3 === 'true') {
        // V3 mode: JSX should work in .js files even without explicit config
        let b2 = await bundle(path.join(dir, 'index-with-jsx.js'), {
          inputFS: overlayFS,
          featureFlags: {
            newJsxConfig: true,
          },
        });
        let js2 = await outputFS.readFile(b2.getBundles()[0].filePath, 'utf8');
        assert(
          js2.includes('React.createElement'),
          'Should transform JSX in .js files in V3 mode',
        );
      } else {
        // V2 mode: JSX should fail in .js files without explicit config
        try {
          await bundle(path.join(dir, 'index-with-jsx.js'), {
            inputFS: overlayFS,
            featureFlags: {
              newJsxConfig: true,
            },
          });
          assert.fail(
            'Should have failed to parse JSX in .js file without explicit config in V2 mode',
          );
        } catch (err) {
          // Expected to fail - JSX should not be enabled for .js files without explicit config in V2
          assert(
            err.message.includes('Unexpected token') ||
              err.message.includes('Expression expected') ||
              err.message.includes('Unexpected'),
            `Expected JSX parsing error, got: ${err.message}`,
          );
        }
      }
    });
  });
});
