import assert from 'assert';
import path from 'path';
import {
  describe,
  it,
  overlayFS,
  fsFixture,
  bundle,
  assertBundles,
  run,
} from '@atlaspack/test-utils';

describe('resolveOptions integration', function () {
  describe('projectRoot option', function () {
    it('should handle absolute projectRoot with bundling', async function () {
      let projectRoot = path.join(__dirname, 'fixtures', 'absolute-project');
      overlayFS.mkdirp(projectRoot);
      await fsFixture(overlayFS, projectRoot)`
      package.json:
        {
          "name": "absolute-project"
        }
      src/index.js:
        export default {
          project: 'absolute-project'
        };
      `;

      let b = await bundle(path.join(projectRoot, 'src/index.js'), {
        projectRoot,
        inputFS: overlayFS,
        outputFS: overlayFS,
        shouldDisableCache: true,
      });

      assertBundles(b, [
        {type: 'js', assets: ['esmodule-helpers.js', 'index.js']},
      ]);
      let output = await run(b);
      assert.strictEqual(output.default.project, 'absolute-project');
    });

    it('should throw error when relative projectRoot is specified', async function () {
      let testDir = path.join(__dirname, 'fixtures', 'relative-test');
      let projectDir = path.join(testDir, 'test-project');
      let relativeRoot = path.relative(process.cwd(), projectDir);

      overlayFS.mkdirp(testDir);
      await fsFixture(overlayFS, testDir)`
      test-project/package.json:
        {
          "name": "relative-project"
        }
      test-project/src/app.js:
        export default {
          project: 'relative-project'
        };
      `;

      let didFail = false;
      try {
        await bundle(path.join(projectDir, 'src/app.js'), {
          projectRoot: relativeRoot,
          inputFS: overlayFS,
          outputFS: overlayFS,
          shouldDisableCache: true,
        });
        assert.fail(
          'Expected bundling to fail when relative project root is specified, but it succeeded',
        );
      } catch (error) {
        didFail = true;
        assert(
          error.message.includes(
            'Specified project root must be an absolute path',
          ),
          `Expected error for relative project root, got: ${error.message}`,
        );
      }

      assert(
        didFail,
        'Expected bundle to fail when relative project root is specified',
      );
    });

    it('should handle default projectRoot auto-detection with bundling', async function () {
      let projectRoot = path.join(__dirname, 'fixtures', 'auto-detect');
      overlayFS.mkdirp(projectRoot);
      await fsFixture(overlayFS, projectRoot)`
      yarn.lock:
        # yarn lockfile
      package.json:
        {
          "name": "auto-detect-project"
        }
      nested/src/main.js:
        export default {
          project: 'auto-detect-project'
        };
      `;

      let b = await bundle(path.join(projectRoot, 'nested/src/main.js'), {
        inputFS: overlayFS,
        outputFS: overlayFS,
        shouldDisableCache: true,
      });

      assertBundles(b, [
        {type: 'js', assets: ['esmodule-helpers.js', 'main.js']},
      ]);
      let output = await run(b);
      assert.strictEqual(output.default.project, 'auto-detect-project');
    });
  });

  describe('plugin resolution', function () {
    it.v2(
      'should fail with default project root for plugin resolution',
      async function () {
        let fixtureRoot = path.join(__dirname, 'fixtures', 'plugin-resolution');
        let nestedProjectDir = path.join(fixtureRoot, 'nested');
        let entryPath = path.join(nestedProjectDir, 'src', 'index.js');

        overlayFS.mkdirp(fixtureRoot);
        await fsFixture(overlayFS, fixtureRoot)`
      plugins/foo.js:
        module.exports = {};
      nested/yarn.lock:
        # Yarn lockfile v1
      nested/package.json:
        {
          "name": "nested-app"
        }
      nested/.parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "transformers": {
            "*.js": ["file:../plugins/foo.js"]
          }
        }
      nested/src/index.js:
        export default { message: 'hello world' };
      `;

        // Default behaviour finds nested/yarn.lock and sets projectRoot to nested/
        // The file: path "../plugins/foo.js" goes outside the auto-detected boundary
        let didFail = false;
        try {
          await bundle(entryPath, {
            inputFS: overlayFS,
            outputFS: overlayFS,
            shouldDisableCache: true,
          });
          assert.fail(
            'Expected bundling to fail with auto-detected project root, but it succeeded',
          );
        } catch (error) {
          didFail = true;
          // Verify this is the expected plugin resolution error
          assert(
            error.message.includes(
              'Cannot find Atlaspack plugin "file:../plugins/foo.js"',
            ),
            `Expected plugin resolution error for file:../plugins/foo.js, got: ${error.message}`,
          );
        }

        assert(
          didFail,
          'Expected bundle to fail when auto-detection causes wrong file: plugin resolution',
        );

        // Passes with specified projectRoot
        let b = await bundle(entryPath, {
          projectRoot: fixtureRoot,
          inputFS: overlayFS,
          outputFS: overlayFS,
          shouldDisableCache: true,
        });

        assertBundles(b, [
          {type: 'js', assets: ['esmodule-helpers.js', 'index.js']},
        ]);
        let output = await run(b);
        assert.strictEqual(output.default.message, 'hello world');
      },
    );
  });
});
