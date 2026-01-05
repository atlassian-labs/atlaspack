// Reproduction for https://github.com/parcel-bundler/parcel/issues/10175
// Issue: Nested re-exports with namespace imports are not properly resolved during scope hoisting

import {bundle, fsFixture, overlayFS, run} from '@atlaspack/test-utils';
import path from 'path';

describe('parcel bug 10175 - nested re-exports with namespace imports', () => {
  it.skip('should work with old zod', async () => {
    const dir = path.join(__dirname, 'parcel-bug-10175-old-zod');
    await overlayFS.mkdirp(dir);
    await fsFixture(overlayFS, dir)`
      index.js:
        import {z} from 'zod';

        console.log(z.object({}));

      package.json:
        {
          "@atlaspack/resolver-default": {
            "packageExports": true
          }
        }

      yarn.lock: {}
    `;
    const b = await bundle(path.join(dir, 'index.js'), {
      inputFS: overlayFS,
      outputFS: overlayFS,
      mode: 'production',
      defaultTargetOptions: {
        shouldScopeHoist: true,
      },
    });
    await run(b);
  });

  it.skip('should work with new zod', async () => {
    const dir = path.join(__dirname, 'parcel-bug-10175-new-zod');
    await overlayFS.mkdirp(dir);
    await fsFixture(overlayFS, dir)`
      index.js:
        import {z} from 'zod-new/v4';

        console.log(z.object({}));

      package.json:
        {
          "@atlaspack/resolver-default": {
            "packageExports": true
          }
        }

      yarn.lock: {}
    `;
    const b = await bundle(path.join(dir, 'index.js'), {
      inputFS: overlayFS,
      outputFS: overlayFS,
      mode: 'production',
      defaultTargetOptions: {
        shouldScopeHoist: true,
      },
    });
    await run(b);
  });

  describe('minimal repro - simpler case', () => {
    it('should handle simple namespace re-exports', async () => {
      const dir = path.join(__dirname, 'parcel-bug-10175-minimal-simple');
      await overlayFS.mkdirp(dir);

      await fsFixture(overlayFS, dir)`
        schemas.js:
          export const ZodObject = function() { return 'ZodObject'; };

        external.js:
          export * from './schemas.js';

        index.js:
          import * as z from './external.js';
          export { z };
          console.log(z.ZodObject());

        package.json:
          {}

        yarn.lock: {}
      `;

      const b = await bundle(path.join(dir, 'index.js'), {
        inputFS: overlayFS,
        outputFS: overlayFS,
        mode: 'production',
        defaultTargetOptions: {
          shouldScopeHoist: true,
        },
      });

      await run(b);
    });

    it('should handle nested namespace re-exports (core issue)', async () => {
      const dir = path.join(__dirname, 'parcel-bug-10175-minimal-nested');
      await overlayFS.mkdirp(dir);

      // This reproduces the exact issue: multiple levels of namespace re-exports
      // similar to zod-new/v4's structure:
      // - v4.js imports from classic.js and re-exports
      // - classic.js imports * as z from external.js and exports { z }
      // - external.js does export * from other modules
      await fsFixture(overlayFS, dir)`
        core.js:
          export const CoreThing = function() { return 'CoreThing'; };

        schemas.js:
          export const ZodObject = function() { return 'ZodObject'; };

        external.js:
          export * as core from './core.js';
          export * from './schemas.js';

        classic.js:
          import * as z from './external.js';
          export { z };
          export * from './external.js';

        v4.js:
          import z4 from './classic.js';
          export * from './classic.js';
          export default z4;

        index.js:
          import {z} from './v4.js';
          console.log(z.ZodObject());

        package.json:
          {}

        yarn.lock: {}
      `;

      const b = await bundle(path.join(dir, 'index.js'), {
        inputFS: overlayFS,
        outputFS: overlayFS,
        mode: 'production',
        defaultTargetOptions: {
          shouldScopeHoist: true,
        },
      });

      await run(b);
    });
  });
});
