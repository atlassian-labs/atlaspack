// @flow strict-local

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

    it('should handle relative projectRoot with bundling', async function () {
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

      let b = await bundle(path.join(projectDir, 'src/app.js'), {
        projectRoot: relativeRoot,
        inputFS: overlayFS,
        outputFS: overlayFS,
        shouldDisableCache: true,
      });

      assertBundles(b, [
        {type: 'js', assets: ['app.js', 'esmodule-helpers.js']},
      ]);
      let output = await run(b);
      assert.strictEqual(output.default.project, 'relative-project');
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

  describe('gitRoot option', function () {
    it('should handle explicit gitRoot with monorepo cross-package imports', async function () {
      let monorepoRoot = path.join(__dirname, 'fixtures', 'monorepo');
      let projectDir = path.join(monorepoRoot, 'packages', 'frontend');

      overlayFS.mkdirp(monorepoRoot);
      await fsFixture(overlayFS, monorepoRoot)`
      .git/config:
        [core]
          repositoryformatversion = 0
      packages/shared/index.js:
        export const utils = {
          formatMessage: (msg) => "[SHARED] " + msg
        };
      packages/frontend/src/app.js:
        import {utils} from '../../shared/index.js';
        export default {
          message: utils.formatMessage('Frontend App')
        };
      `;

      let b = await bundle(path.join(projectDir, 'src/app.js'), {
        projectRoot: projectDir,
        gitRoot: monorepoRoot,
        inputFS: overlayFS,
        outputFS: overlayFS,
        shouldDisableCache: true,
      });

      assertBundles(b, [
        {type: 'js', assets: ['app.js', 'esmodule-helpers.js', 'index.js']},
      ]);
      let output = await run(b);
      assert.strictEqual(output.default.message, '[SHARED] Frontend App');
    });

    it('should handle gitRoot auto-detection with bundling', async function () {
      let projectRoot = path.join(__dirname, 'fixtures', 'auto-git');
      overlayFS.mkdirp(projectRoot);
      await fsFixture(overlayFS, projectRoot)`
      .git/config:
        [core]
          repositoryformatversion = 0
      package.json:
        {
          "name": "auto-git-project"
        }
      src/main.js:
        export default {
          project: 'auto-git-project'
        };
      `;

      let b = await bundle(path.join(projectRoot, 'src/main.js'), {
        inputFS: overlayFS,
        outputFS: overlayFS,
        shouldDisableCache: true,
      });

      assertBundles(b, [
        {type: 'js', assets: ['esmodule-helpers.js', 'main.js']},
      ]);
      let output = await run(b);
      assert.strictEqual(output.default.project, 'auto-git-project');
    });
  });
});
