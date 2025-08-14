import assert from 'assert';
import path from 'path';
import {
  bundler,
  describe,
  fsFixture,
  it,
  overlayFS,
  run,
  getNextBuild,
} from '@atlaspack/test-utils';
import tempy from 'tempy';

describe.v2('symbol propagation', () => {
  it('should handle removed assets from previous failed builds', async () => {
    await fsFixture(overlayFS, __dirname)`
        broken.js:
            module.exports = require('./missing.js');
        working.js:
            module.exports = 'ITS WORKING';
        index.js:
            module.exports = require('./broken.js');`;

    let b = bundler(path.join(__dirname, 'index.js'), {
      inputFS: overlayFS,
      shouldDisableCache: false,
    });

    await assert.rejects(() => b.run(), {
      message: `Failed to resolve './missing.js' from './broken.js'`,
    });

    await overlayFS.writeFile(
      path.join(__dirname, 'index.js'),
      `module.exports = require('./working.js');`,
    );

    let {bundleGraph} = await b.run();

    assert(await run(bundleGraph), 'ITS WORKING');
  });

  it('should handle assets referenced elsewhere from previous failed builds', async () => {
    let dir = tempy.directory();
    let testFs = overlayFS;
    await testFs.mkdirp(dir);
    await fsFixture(testFs, dir)`
      yarn.lock: {}
                
      node_modules
        @atlaskit
          primitives
            package.json: {}

            index.ts:
              export function xcss() {
                return 'xcss';
              }
      
            compiled
              index.ts:
                export function notXcss() {
                  return 'notXcss';
                }

      index.ts:
        import { xcss } from '@atlaskit/primitives';
        import Component from './component.ts';
        console.log(xcss());
        console.log(Component());

      component.ts:
        import { notXcss } from '@atlaskit/primitives/compiled';

        export default function Component() {
          return "Hi! " + notXcss();
        }
    `;

    const rewriteImport = async (oldImport: string, newImport: string) => {
      const filePath = path.join(dir, 'index.ts');
      let code = await testFs.readFile(filePath, 'utf8');
      // Replace only the import statement that matches the oldImport string
      const importRegex = new RegExp(
        `(import\\s+[^'"]+\\s+from\\s+)'${oldImport}'`,
      );
      code = code.replace(importRegex, `$1'${newImport}'`);
      await testFs.writeFile(filePath, code, 'utf8');
    };

    let b = bundler(path.join(dir, 'index.ts'), {
      inputFS: testFs,
      shouldDisableCache: false,
    });
    let subscription = await b.watch();
    try {
      let result = await getNextBuild(b);
      assert(result.type === 'buildSuccess');

      await rewriteImport(
        '@atlaskit/primitives',
        '@atlaskit/primitives/compiled',
      );

      result = await getNextBuild(b);

      // The second build should fail because the import is now invalid
      assert(result.type === 'buildFailure');
      assert(
        result.diagnostics[0]?.message.includes("does not export 'xcss'"),
        'Expected an error that xcss is not exported',
      );

      await rewriteImport(
        '@atlaskit/primitives/compiled',
        '@atlaskit/primitives',
      );

      // The third build should succeed because the import is now valid
      result = await getNextBuild(b);
      assert(result.type === 'buildSuccess', 'Expected the build to succeed');
    } finally {
      subscription.unsubscribe();
      await testFs.rimraf(dir);
    }
  });
});
