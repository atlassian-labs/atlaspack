import assert from 'assert';
import path, {join} from 'path';
import {
  bundle,
  describe,
  it,
  run,
  ncp,
  overlayFS,
  outputFS,
  fsFixture,
  inputFS,
  findAsset,
} from '@atlaspack/test-utils';

describe('resolver', function () {
  it('should support resolving tilde in monorepo packages', async function () {
    let b = await bundle(
      path.join(
        __dirname,
        '/integration/resolve-tilde-monorepo/client/src/index.js',
      ),
    );

    let output = await run(b);
    assert.strictEqual(output.default, 1234);
  });

  it('should support node: prefix for node_modules', async function () {
    let b = await bundle(
      path.join(__dirname, '/integration/resolve-node-prefix/src/index.js'),
    );

    let output = await run(b);
    assert.strictEqual(
      output.default,
      '6a2da20943931e9834fc12cfe5bb47bbd9ae43489a30726962b576f4e3993e50',
    );
  });

  it('should correctly resolve tilde in node_modules', async function () {
    let b = await bundle(
      path.join(__dirname, '/integration/resolve-tilde-nodemodules/index.js'),
    );

    let output = await run(b);
    assert.strictEqual(output.default, 1234);
  });

  it('should fall back to index.js if the resolved `main` file does not exist', async function () {
    let b = await bundle(
      path.join(
        __dirname,
        '/integration/resolve-index-fallback/incorrect-entry.js',
      ),
    );

    let output = await run(b);
    assert.strictEqual(output.default, 42);
  });

  it('should fall back to index.js if there is no `main` field at all', async function () {
    let b = await bundle(
      path.join(__dirname, '/integration/resolve-index-fallback/no-entry.js'),
    );

    let output = await run(b);
    assert.strictEqual(output.default, 42);
  });

  it.v2(
    'should print a diagnostic when a configured target field will overwrite an entry',
    async function () {
      let errorThrows = 0;
      const overwriteDirs = ['browser', 'app', 'main', 'module'];
      for (const currDir of overwriteDirs) {
        try {
          await bundle(
            path.join(
              __dirname,
              `integration/target-overwrite-source/${currDir}`,
            ),
          );
        } catch (e: any) {
          errorThrows++;
          let pkg = JSON.parse(
            await overlayFS.readFile(
              path.join(
                __dirname,
                `integration/target-overwrite-source/${currDir}/package.json`,
              ),
            ),
          );
          assert.deepEqual(
            e.diagnostics[0].message,
            `Target "${currDir}" is configured to overwrite entry "${path.normalize(
              `test/integration/target-overwrite-source/${currDir}/${pkg.source}`,
            )}".`,
          );
        }
      }
      assert.deepEqual(errorThrows, overwriteDirs.length);
    },
  );

  it.v2('should throw an error on Webpack loader imports', async function () {
    let didThrow = false;
    try {
      await bundle(
        path.join(
          __dirname,
          '/integration/webpack-import-syntax-error/index.js',
        ),
      );
    } catch (e: any) {
      didThrow = true;
      assert.equal(
        e.diagnostics[1].message,
        `The import path: node-loader!./index.js is using webpack specific loader import syntax, which isn't supported by Parcel.`,
      );
    }

    assert(didThrow);
  });

  it.v2(
    'should throw an error with codeframe on invalid js import',
    async function () {
      let didThrow = false;
      try {
        await bundle(
          path.join(__dirname, '/integration/js-invalid-import/index.js'),
        );
      } catch (e: any) {
        didThrow = true;

        assert(
          e.diagnostics[0].message.startsWith(
            `Failed to resolve './doesnotexisstt' from `,
          ),
        );

        assert.deepEqual(e.diagnostics[0].codeFrames[0].codeHighlights[0], {
          message: undefined,
          start: {line: 1, column: 8},
          end: {line: 1, column: 25},
        });
      }

      assert(didThrow);
    },
  );

  it.v2(
    'should throw an error with codeframe on invalid css import',
    async function () {
      let didThrow = false;
      try {
        await bundle(
          path.join(__dirname, '/integration/css-invalid-import/index.css'),
        );
      } catch (e: any) {
        didThrow = true;

        assert(
          e.diagnostics[0].message.startsWith(
            `Failed to resolve './thisdoesnotexist.css' from `,
          ),
        );

        assert.deepEqual(e.diagnostics[0].codeFrames[0].codeHighlights[0], {
          message: undefined,
          start: {line: 1, column: 9},
          end: {line: 1, column: 32},
        });
      }

      assert(didThrow);
    },
  );

  it.v2(
    'Should return codeframe with hints when package.json is invalid',
    async function () {
      let didThrow = false;
      try {
        await bundle(
          path.join(
            __dirname,
            '/integration/resolver-invalid-pkgjson/index.js',
          ),
        );
      } catch (e: any) {
        didThrow = true;

        assert.equal(
          e.diagnostics[1].message,
          `Could not load './entryx.js' from module 'invalid-module' found in package.json#main`,
        );

        assert.deepEqual(e.diagnostics[1].codeFrames[0].codeHighlights[0], {
          end: {
            column: 25,
            line: 4,
          },
          message: "'./entryx.js' does not exist, did you mean './entry.js'?'",
          start: {
            column: 13,
            line: 4,
          },
        });
      }

      assert(didThrow);
    },
  );

  it.v2(
    'Should suggest alternative filenames for relative imports',
    async function () {
      let threw = 0;

      try {
        await bundle(
          path.join(
            __dirname,
            '/integration/resolver-alternative-relative/a.js',
          ),
        );
      } catch (e: any) {
        threw++;

        assert.equal(
          e.diagnostics[1].message,
          `Cannot load file './test/teste.js' in './integration/resolver-alternative-relative'.`,
        );

        assert.equal(
          e.diagnostics[1].hints[0],
          `Did you mean '__./test/test.js__'?`,
        );
      }

      try {
        await bundle(
          path.join(
            __dirname,
            '/integration/resolver-alternative-relative/b.js',
          ),
        );
      } catch (e: any) {
        threw++;

        assert.equal(
          e.diagnostics[1].message,
          `Cannot load file './aa.js' in './integration/resolver-alternative-relative'.`,
        );

        assert.equal(e.diagnostics[1].hints[0], `Did you mean '__./a.js__'?`);
      }

      try {
        await bundle(
          path.join(
            __dirname,
            '/integration/resolver-alternative-relative/test/test.js',
          ),
        );
      } catch (e: any) {
        threw++;

        assert.equal(
          e.diagnostics[1].message,
          `Cannot load file '../../a.js' in './integration/resolver-alternative-relative/test'.`,
        );

        assert.equal(e.diagnostics[1].hints[0], `Did you mean '__../a.js__'?`);
      }

      assert.equal(threw, 3);
    },
  );

  it.v2(
    'Should suggest alternative modules for module imports',
    async function () {
      let threw = false;

      try {
        await bundle(
          path.join(
            __dirname,
            '/integration/resolver-alternative-module/index.js',
          ),
        );
      } catch (e: any) {
        threw = true;

        assert.equal(
          e.diagnostics[1].message,
          `Cannot find module '@baebal/core'`,
        );

        assert.equal(
          e.diagnostics[1].hints[0],
          `Did you mean '__@babel/core__'?`,
        );
      }

      assert(threw);
    },
  );

  it('should resolve packages to packages through the alias field', async function () {
    let b = await bundle(
      path.join(__dirname, '/integration/alias/package-to-package.js'),
    );

    let output = await run(b);
    assert.strictEqual(output.default, 3);
  });

  it('should resolve packages to local files through the alias field', async function () {
    let b = await bundle(
      path.join(__dirname, '/integration/alias/package-to-local.js'),
    );

    let output = await run(b);
    assert.strictEqual(output.default, 'bar');
  });

  it('should exclude local files using the alias field', async function () {
    let b = await bundle(
      path.join(__dirname, '/integration/alias/exclude-local.js'),
    );

    let output = await run(b);
    assert.deepEqual(output.default, {});
  });

  it('should exclude packages using the alias field', async function () {
    let b = await bundle(
      path.join(__dirname, '/integration/alias/exclude-package.js'),
    );

    let output = await run(b);
    assert.deepEqual(output.default, {});
  });

  it('should support symlinked node_modules structure', async function () {
    const rootDir = path.join(
      __dirname,
      'integration/resolve-symlinked-node_modules-structure',
    );

    await overlayFS.mkdirp(rootDir);
    await ncp(rootDir, rootDir);

    await outputFS.symlink(
      path.join(
        rootDir,
        'node_modules/.origin/library@1.0.0/node_modules/library',
      ),
      path.join(rootDir, 'node_modules/library'),
    );
    await outputFS.symlink(
      path.join(
        rootDir,
        'node_modules/.origin/library-dep@1.0.0/node_modules/library-dep',
      ),
      path.join(
        rootDir,
        'node_modules/.origin/library@1.0.0/node_modules/library-dep',
      ),
    );

    let b = await bundle(
      path.join(
        __dirname,
        '/integration/resolve-symlinked-node_modules-structure/index.js',
      ),
      {
        inputFS: overlayFS,
        outputFS,
      },
    );

    let output = await run(b);
    assert.strictEqual(output.default, 42);
  });

  it.v2('should support symlinked monorepos structure', async function () {
    const rootDir = path.join(
      __dirname,
      'integration/resolve-symlinked-monorepos',
    );

    await overlayFS.mkdirp(rootDir);
    await ncp(rootDir, rootDir);

    await outputFS.symlink(
      path.join(rootDir, 'packages/library'),
      path.join(rootDir, 'packages/app/node_modules/library'),
    );
    await outputFS.symlink(
      path.join(rootDir, 'node_modules/.origin/pkg@1.0.0/node_modules/pkg'),
      path.join(rootDir, 'packages/app/node_modules/pkg'),
    );
    await outputFS.symlink(
      path.join(rootDir, 'node_modules/.origin/pkg@1.0.0/node_modules/pkg'),
      path.join(rootDir, 'packages/library/node_modules/pkg'),
    );

    let b = await bundle(
      path.join(
        __dirname,
        '/integration/resolve-symlinked-monorepos/packages/app/index.js',
      ),
      {
        inputFS: overlayFS,
        outputFS,
      },
    );

    let output = await run(b);
    assert.strictEqual(output.default, 2);
  });

  it.v2('should support very long dependency specifiers', async function () {
    this.timeout(8000);

    let inputDir = path.join(__dirname, 'input');

    await outputFS.mkdirp(inputDir);
    await outputFS.writeFile(
      path.join(inputDir, 'index.html'),
      `<img src="data:image/jpeg;base64,/9j/${'A'.repeat(200000)}">`,
    );

    await bundle(path.join(inputDir, 'index.html'), {
      inputFS: overlayFS,
    });
  });

  it.v2('should support empty dependency specifiers', async function () {
    await assert.rejects(
      () =>
        bundle(
          path.join(__dirname, '/integration/resolve-empty-specifier/index.js'),
        ),
      {
        message: `Failed to resolve '' from './integration/resolve-empty-specifier/index.js'`,
      },
    );
  });

  it('should support package exports config option', async () => {
    let b = await bundle(
      path.join(__dirname, '/integration/resolve-exports/index.js'),
    );

    let output = await run(b);
    assert.strictEqual(output.default, 'hello bar');
  });

  it('should support the development and production import conditions', async () => {
    let b = await bundle(
      path.join(__dirname, '/integration/resolve-mode-condition/index.js'),
      {mode: 'development'},
    );

    let output = await run(b);
    assert.strictEqual(output.default, 'development');

    b = await bundle(
      path.join(__dirname, '/integration/resolve-mode-condition/index.js'),
      {mode: 'production'},
    );

    output = await run(b);
    assert.strictEqual(output.default, 'production');
  });

  describe('resolver sideEffects defaults', function () {
    const dir = join(__dirname, 'tmp');

    beforeEach(async function () {
      await inputFS.rimraf(dir);
    });

    afterEach(async function () {
      await inputFS.rimraf(dir);
    });

    it('should default sideEffects to true when custom resolver does not specify it', async function () {
      // This test verifies the fix for the bug where custom resolvers that don't specify
      // sideEffects would get false by default in V3, causing issues with asset graph construction.
      // The fix ensures sideEffects defaults to true, matching V2 behavior.

      await fsFixture(inputFS, dir)`
        custom-resolver-sideeffects-default
          package.json:
            {
              "name": "custom-resolver-sideeffects-default",
              "version": "1.0.0"
            }

          .parcelrc:
            {
              "extends": "@atlaspack/config-default",
              "resolvers": ["./custom-resolver.js", "..."]
            }

          yarn.lock:

          index.js:
            // This file imports a module that will be resolved by a custom resolver
            // The custom resolver doesn't specify sideEffects, so it should default to true
            import value from 'custom-module';

            export default value;

          custom-module.js:
            // This module is resolved by the custom resolver
            export default 42;

          custom-resolver.js:
            const {Resolver} = require('@atlaspack/plugin');
            const path = require('path');

            /**
             * Custom resolver that doesn't specify sideEffects.
             * This tests that sideEffects defaults to true in V3.
             */
            module.exports = new Resolver({
              async resolve({specifier}) {
                if (specifier === 'custom-module') {
                  return {
                    filePath: path.join(__dirname, 'custom-module.js'),
                    // Note: sideEffects is NOT specified - should default to true
                  };
                }

                return null;
              },
            });
      `;

      let b = await bundle(
        join(dir, 'custom-resolver-sideeffects-default/index.js'),
        {
          inputFS,
        },
      );

      let output = await run(b);
      assert.strictEqual(output.default, 42);

      let customModule = findAsset(b, 'custom-module.js');
      assert.equal(customModule.sideEffects, true);
    });

    it('should default sideEffects to true when using the native atlaspack resolver', async function () {
      // This test verifies that when the built-in Atlaspack resolver resolves Node builtins
      // without polyfills to _empty.js, sideEffects defaults to true, preventing symbol
      // propagation errors during scope hoisting.
      //
      // Without the fix (passing false), this test would fail with:
      // "does not export 'Buffer'" or similar errors during symbol propagation.

      await fsFixture(inputFS, dir)`
        native-resolver
          lib.js:
            var fs;
            try {
              // fs is a Node builtin, so it will be resolved to _empty.js
              fs = require('fs');
            } catch (e) {
            }

          index.js:
            import './lib.js';
            sideEffectNoop();
      `;

      let b = await bundle(join(dir, 'native-resolver/index.js'), {
        inputFS,
        mode: 'production', // Enable scope hoisting to trigger symbol propagation
      });

      let empty = findAsset(b, '_empty.js');
      assert.equal(empty.sideEffects, true);
    });
  });

  describe.v3('unstable_alias', function () {
    // unstable_alias is currently only supported in v3
    it('should resolve aliases set by unstable_alias in .parcelrc', async function () {
      await fsFixture(overlayFS, __dirname)`
        unstable-alias-test
          .parcelrc:
            {
              "extends": "@atlaspack/config-default",
               "unstable_alias": {"my-alias": "./test.js"}
            }

          package.json:
            {
              "name": "unstable-alias-test"
            }

          yarn.lock:

          test.js:
            module.exports = 42;

          index.js:
            module.exports = 'hello ' + require('my-alias');
      `;

      let b = await bundle(
        path.join(__dirname, 'unstable-alias-test/index.js'),
        {
          inputFS: overlayFS,
        },
      );

      let output = await run(b);
      assert.equal(output, 'hello 42');
    });

    it('should handle unstable_alias as well as regular alias', async function () {
      await fsFixture(overlayFS, __dirname)`
        unstable-alias-test-2
          .parcelrc:
            {
              "extends": "@atlaspack/config-default",
               "unstable_alias": {"my-alias": "./test.js"}
            }

          package.json:
            {
              "name": "unstable-alias-test",
              "alias": {
                "my-other-alias": "./other.js"
              }
            }

          yarn.lock:

          other.js:
            module.exports = 'other';

          test.js:
            module.exports = 'test';

          index.js:
            module.exports = 'hello ' + require('my-alias') + ' ' + require('my-other-alias');
      `;

      let b = await bundle(
        path.join(__dirname, 'unstable-alias-test-2/index.js'),
        {
          inputFS: overlayFS,
        },
      );

      let output = await run(b);
      assert.equal(output, 'hello test other');
    });
  });
});
