import assert from 'assert';
import path from 'path';
import {
  bundle,
  describe,
  it,
  fsFixture,
  run,
  runBundle,
  overlayFS,
} from '@atlaspack/test-utils';
import {BundleGraph, PackagedBundle} from '@atlaspack/types-internal';

async function assertBundleOutput(
  b: BundleGraph<PackagedBundle>,
  bundleMatch: string,
  expected: any,
) {
  let bundle = b.getBundles().find((b) => b.filePath.includes(bundleMatch));
  let output: any = await runBundle(b, bundle, {});
  assert.deepEqual(output?.default, expected);
}

describe('Entry Points', function () {
  let dir: string = path.join(__dirname, 'entry-points-fixture');

  beforeEach(async function () {
    await overlayFS.rimraf(dir);
    await overlayFS.mkdirp(dir);
  });

  describe('File Entry Points', function () {
    it('should handle single file entry', async () => {
      await fsFixture(overlayFS, dir)`
        index.js:
          export default 'hello from file entry';
      `;

      let b = await bundle(path.join(dir, 'index.js'), {
        inputFS: overlayFS,
      });

      assert.equal(b.getBundles().length, 1);
      let output = await run(b);
      assert.equal(output.default, 'hello from file entry'); // console.log returns undefined, but run() returns an object
    });

    it('should handle multiple file entries', async () => {
      await fsFixture(overlayFS, dir)`
        index.js:
          export default 'main entry';
        alt.js:
          export default 'alt entry';
      `;

      let b = await bundle(
        [path.join(dir, 'index.js'), path.join(dir, 'alt.js')],
        {
          inputFS: overlayFS,
        },
      );

      assert.equal(b.getBundles().length, 2);
      await assertBundleOutput(b, 'index.js', 'main entry');
      await assertBundleOutput(b, 'alt.js', 'alt entry');
    });
  });

  describe('Directory Entry Points', function () {
    it('should handle directory with package.json source field', async () => {
      await fsFixture(overlayFS, dir)`
        test-package
          package.json:
            {"source": "src/index.js"}
          src
            index.js:
              export default 'hello from directory entry';
      `;

      let b = await bundle(path.join(dir, 'test-package'), {
        inputFS: overlayFS,
      });

      assert.equal(b.getBundles().length, 1);
      let output = await run(b);
      assert.equal(output.default, 'hello from directory entry');
    });

    it('should handle directory with package.json source array', async () => {
      await fsFixture(overlayFS, dir)`
        test-package
          package.json:
            {"source": ["src/index.js", "src/alt.js"]}
          src
            index.js:
              export default 'main entry';
            alt.js:
              export default 'alt entry';
      `;

      let b = await bundle(path.join(dir, 'test-package'), {
        inputFS: overlayFS,
      });

      assert.equal(b.getBundles().length, 2);
      await assertBundleOutput(b, 'index', 'main entry');
      await assertBundleOutput(b, 'alt', 'alt entry');
    });

    it('should handle directory with custom targets defined in package.json targets field', async () => {
      await fsFixture(overlayFS, dir)`
        test-package
          yarn.lock:
          package.json:
            {
              "targets": {
                "custom1": {"source": "src/index.js"},
                "custom2": {"source": "src/alt.js"}
              }
            }
          src
            index.js:
              export default 'main entry';
            alt.js:
              export default 'alt entry';
      `;

      let b = await bundle(path.join(dir, 'test-package'), {
        inputFS: overlayFS,
      });

      assert.equal(b.getBundles().length, 2);
      await assertBundleOutput(b, 'index', 'main entry');
      await assertBundleOutput(b, 'alt', 'alt entry');
    });

    it.v2(
      'should handle directory with builtin target defined in package.json targets field',
      async function () {
        await fsFixture(overlayFS, dir)`
        test-package
          yarn.lock:
          package.json:
            {
              "targets": {
                "main": {"source": "src/index.js"},
                "alt": {"source": "src/alt.js"}
              }
            }
          src
            index.js:
              export default 'main entry';
            alt.js:
              export default 'alt entry';
      `;

        let b = await bundle(path.join(dir, 'test-package'), {
          inputFS: overlayFS,
        });

        assert.equal(b.getBundles().length, 2);
        await assertBundleOutput(b, 'index', 'main entry');
        await assertBundleOutput(b, 'alt', 'alt entry');
      },
    );

    it.v2(
      'should handle builtin main target defined targets field',
      async function () {
        await fsFixture(overlayFS, dir)`
        test-package
          yarn.lock:
          package.json:
            {
              "targets": {
                "main": {"source": "src/index.js"}
              }
            }
          src
            index.js:
              export default 'main entry';
      `;

        let b = await bundle(path.join(dir, 'test-package'), {
          inputFS: overlayFS,
        });

        assert.equal(b.getBundles().length, 1);
        await assertBundleOutput(b, 'index', 'main entry');
      },
    );

    it.v2(
      'should build library with main defined in top level package.json field',
      async function () {
        await fsFixture(overlayFS, dir)`
        test-package
          yarn.lock:
          package.json:
            {
              "main": "dist/index.js",
              "source": "src/index.js"
            }
          src
            index.js:
              export default 'main entry';
      `;

        let b = await bundle(path.join(dir, 'test-package'), {
          inputFS: overlayFS,
          defaultTargetOptions: {
            outputFormat: 'commonjs',
            distDir: path.join(dir, 'dist'),
          },
        });

        assert.equal(b.getBundles().length, 1);
        await assertBundleOutput(b, 'index', 'main entry');
      },
    );

    it('should bundle correct sources for each target when given directory entry point', async () => {
      const testDir = path.join(dir, 'monorepo');
      await overlayFS.mkdirp(testDir);

      await fsFixture(overlayFS, testDir)`
        yarn.lock:

        package.json:
          {
            "name": "monorepo-root"
          }

        packages/my-app
          package.json:
            {
              "name": "my-app",
              "main": "index.jsx",
              "targets": {
                "main": false,
                "development": {
                  "source": [
                    "one.js",
                    "two.js"
                  ]
                },
                "production": {
                  "source": [
                    "one.js",
                    "two.js",
                    "three.js",
                    "four.js"
                  ]
                }
              }
            }

          one.js:
            export default 'source one';
          two.js:
            export default 'source two';
          three.js:
            export default 'source three';
          four.js:
            export default 'source four';
      `;

      // Build production target
      let prodTargetBundleGraph = await bundle(
        path.join(testDir, 'packages/my-app'),
        {
          inputFS: overlayFS,
          targets: ['production'],
        },
      );

      assert.equal(prodTargetBundleGraph.getBundles().length, 4);
      await assertBundleOutput(prodTargetBundleGraph, 'one.js', 'source one');
      await assertBundleOutput(prodTargetBundleGraph, 'two.js', 'source two');
      await assertBundleOutput(
        prodTargetBundleGraph,
        'three.js',
        'source three',
      );
      await assertBundleOutput(prodTargetBundleGraph, 'four.js', 'source four');

      // Build development target
      let devTargetBundleGraph = await bundle(
        path.join(testDir, 'packages/my-app'),
        {
          inputFS: overlayFS,
          targets: ['development'],
        },
      );

      assert.equal(devTargetBundleGraph.getBundles().length, 2);
      await assertBundleOutput(devTargetBundleGraph, 'one.js', 'source one');
      await assertBundleOutput(devTargetBundleGraph, 'two.js', 'source two');
    });

    it('should prefer targets over source when both are present', async () => {
      await fsFixture(overlayFS, dir)`
        test-package
          package.json:
            {
              "source": "src/fallback.js",
              "targets": {
                "production": {"source": "src/index.js"}
              }
            }
          src
            index.js:
              export default 'target entry';
            fallback.js:
              export default 'source fallback';
      `;

      let b = await bundle(path.join(dir, 'test-package'), {
        inputFS: overlayFS,
      });

      // Should use targets, not source
      assert.equal(b.getBundles().length, 1);
      let output = await run(b);
      assert.equal(output.default, 'target entry');
    });

    it('should handle mixed file and directory entries', async () => {
      await fsFixture(overlayFS, dir)`
        test-package
          package.json:
            {"source": "src/index.js"}
          src
            index.js:
              export default 'directory entry';
        standalone.js:
          export default 'file entry';
      `;

      let b = await bundle(
        [path.join(dir, 'test-package'), path.join(dir, 'standalone.js')],
        {
          inputFS: overlayFS,
        },
      );

      assert.equal(b.getBundles().length, 2);
      await assertBundleOutput(b, 'index.js', 'directory entry');
      await assertBundleOutput(b, 'standalone.js', 'file entry');
    });

    it.v2('should handle complex targets configuration', async function () {
      await fsFixture(overlayFS, dir)`
        test-package
          package.json:
            {
              "targets": {
                "browser": {
                  "source": "src/browser.js",
                  "outputFormat": "esmodule"
                },
                "node": {
                  "source": "src/node.js",
                  "outputFormat": "commonjs"
                }
              }
            }
          src
            browser.js:
              export default 'browser entry';
            node.js:
              export default 'node entry';
      `;

      let b = await bundle(path.join(dir, 'test-package'), {
        inputFS: overlayFS,
      });

      assert.equal(b.getBundles().length, 2);
      await assertBundleOutput(b, 'browser.js', 'browser entry');
      await assertBundleOutput(b, 'node.js', 'node entry');
    });

    it('should ignore disabled targets set to false', async () => {
      await fsFixture(overlayFS, dir)`
        test-package
          package.json:
            {
              "targets": {
                "main": false,
                "development": {
                  "source": "src/dev.js"
                },
                "production": {
                  "source": "src/prod.js"
                }
              }
            }
          src
            dev.js:
              export default 'dev entry';
            prod.js:
              export default 'prod entry';
      `;

      let b = await bundle(path.join(dir, 'test-package'), {
        inputFS: overlayFS,
      });

      // Should have 2 bundles (development and production), main should be ignored
      assert.equal(b.getBundles().length, 2);
      let bundles = b.getBundles();
      // Verify we don't have a bundle for the disabled 'main' target
      assert(!bundles.some((bundle) => bundle.name === 'main'));
      await assertBundleOutput(b, 'dev.js', 'dev entry');
      await assertBundleOutput(b, 'prod.js', 'prod entry');
    });

    it('should handle disabled targets with source array', async () => {
      await fsFixture(overlayFS, dir)`
        test-package
          package.json:
            {
              "targets": {
                "main": false,
                "development": {
                  "source": ["src/dev1.js", "src/dev2.js"]
                },
                "production": {
                  "source": ["src/prod1.js", "src/prod2.js"]
                }
              }
            }
          src
            dev1.js:
              export default 'dev entry 1';
            dev2.js:
              export default 'dev entry 2';
            prod1.js:
              export default 'prod entry 1';
            prod2.js:
              export default 'prod entry 2';
      `;

      let b = await bundle(path.join(dir, 'test-package'), {
        inputFS: overlayFS,
      });

      // Should handle array sources for each enabled target
      assert.equal(b.getBundles().length, 4);
      await assertBundleOutput(b, 'dev1.js', 'dev entry 1');
      await assertBundleOutput(b, 'dev2.js', 'dev entry 2');
      await assertBundleOutput(b, 'prod1.js', 'prod entry 1');
      await assertBundleOutput(b, 'prod2.js', 'prod entry 2');
    });

    it('should error when directory has no package.json', async () => {
      await fsFixture(overlayFS, dir)`
        empty-dir
          some-file.txt:
            content
      `;

      await assert.rejects(
        () => bundle(path.join(dir, 'empty-dir'), {inputFS: overlayFS}),
        /Could not find entry/,
      );
    });

    it('should error when package.json has no source or targets', async () => {
      await fsFixture(overlayFS, dir)`
        test-package
          package.json:
            {"name": "test-package"}
          src
            index.js:
              console.log('unused entry');
      `;

      await assert.rejects(
        () => bundle(path.join(dir, 'test-package'), {inputFS: overlayFS}),
        /Could not find entry/,
      );
    });

    it('should error when source file does not exist', async () => {
      await fsFixture(overlayFS, dir)`
        test-package
          package.json:
            {"source": "src/missing.js"}
          src
            index.js:
              console.log('existing but unused');
      `;

      await assert.rejects(
        () => bundle(path.join(dir, 'test-package'), {inputFS: overlayFS}),
        /does not exist/,
      );
    });

    it('should handle invalid package.json gracefully', async () => {
      await fsFixture(overlayFS, dir)`
        test-package
          package.json:
            {invalid json
          src
            index.js:
              console.log('unused entry');
      `;

      await assert.rejects(
        () => bundle(path.join(dir, 'test-package'), {inputFS: overlayFS}),
        /Error parsing.*package.json/,
      );
    });
  });
});
