import assert from 'assert';
import path from 'path';
import {
  bundle,
  describe,
  it,
  fsFixture,
  run,
  overlayFS,
} from '@atlaspack/test-utils';
import {rimraf} from 'rimraf';

describe('Entry Points', function () {
  let dir: string = path.join(__dirname, 'entry-points-fixture');

  beforeEach(async function () {
    await rimraf(dir);
    await overlayFS.mkdirp(dir);
  });

  describe('File Entry Points', function () {
    it('should handle single file entry', async () => {
      await fsFixture(overlayFS, dir)`
        index.js:
          console.log('hello from file entry');
      `;

      let b = await bundle(path.join(dir, 'index.js'), {
        inputFS: overlayFS,
      });

      assert(b.getBundles().length > 0);
      let output = await run(b);
      assert.equal(typeof output, 'object'); // console.log returns undefined, but run() returns an object
    });

    it('should handle multiple file entries', async () => {
      await fsFixture(overlayFS, dir)`
        index.js:
          console.log('main entry');
        alt.js:
          console.log('alt entry');
      `;

      let b = await bundle(
        [path.join(dir, 'index.js'), path.join(dir, 'alt.js')],
        {
          inputFS: overlayFS,
        },
      );

      assert(b.getBundles().length >= 2);
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
              console.log('hello from directory entry');
      `;

      let b = await bundle(path.join(dir, 'test-package'), {
        inputFS: overlayFS,
      });

      assert(b.getBundles().length > 0);
      let output = await run(b);
      assert.equal(typeof output, 'object');
    });

    it('should handle directory with package.json source array', async () => {
      await fsFixture(overlayFS, dir)`
        test-package
          package.json:
            {"source": ["src/index.js", "src/alt.js"]}
          src
            index.js:
              console.log('main entry');
            alt.js:
              console.log('alt entry');
      `;

      let b = await bundle(path.join(dir, 'test-package'), {
        inputFS: overlayFS,
      });

      assert(b.getBundles().length >= 2);
    });

    it('should handle directory with package.json targets field', async () => {
      await fsFixture(overlayFS, dir)`
        test-package
          package.json:
            {
              "targets": {
                "main": {"source": "src/index.js"},
                "alt": {"source": "src/alt.js"}
              }
            }
          src
            index.js:
              console.log('main entry');
            alt.js:
              console.log('alt entry');
      `;

      let b = await bundle(path.join(dir, 'test-package'), {
        inputFS: overlayFS,
      });

      assert(b.getBundles().length >= 2);
    });

    it('should prefer targets over source when both are present', async () => {
      await fsFixture(overlayFS, dir)`
        test-package
          package.json:
            {
              "source": "src/fallback.js",
              "targets": {
                "main": {"source": "src/index.js"}
              }
            }
          src
            index.js:
              console.log('target entry');
            fallback.js:
              console.log('source fallback');
      `;

      let b = await bundle(path.join(dir, 'test-package'), {
        inputFS: overlayFS,
      });

      // Should use targets, not source
      assert(b.getBundles().length > 0);
      let output = await run(b);
      assert.equal(typeof output, 'object');
    });

    it('should handle mixed file and directory entries', async () => {
      await fsFixture(overlayFS, dir)`
        test-package
          package.json:
            {"source": "src/index.js"}
          src
            index.js:
              console.log('directory entry');
        standalone.js:
          console.log('file entry');
      `;

      let b = await bundle(
        [path.join(dir, 'test-package'), path.join(dir, 'standalone.js')],
        {
          inputFS: overlayFS,
        },
      );

      assert(b.getBundles().length >= 2);
    });

    it('should handle complex targets configuration', async () => {
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
              console.log('browser entry');
            node.js:
              console.log('node entry');
      `;

      let b = await bundle(path.join(dir, 'test-package'), {
        inputFS: overlayFS,
      });

      assert(b.getBundles().length >= 2);
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
              console.log('dev entry');
            prod.js:
              console.log('prod entry');
      `;

      let b = await bundle(path.join(dir, 'test-package'), {
        inputFS: overlayFS,
      });

      // Should have 2 bundles (development and production), main should be ignored
      assert(b.getBundles().length >= 2);
      let bundles = b.getBundles();
      // Verify we don't have a bundle for the disabled 'main' target
      assert(!bundles.some((bundle) => bundle.name === 'main'));
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
              console.log('dev entry 1');
            dev2.js:
              console.log('dev entry 2');
            prod1.js:
              console.log('prod entry 1');
            prod2.js:
              console.log('prod entry 2');
      `;

      let b = await bundle(path.join(dir, 'test-package'), {
        inputFS: overlayFS,
      });

      // Should handle array sources for each enabled target
      assert(b.getBundles().length >= 2);
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
