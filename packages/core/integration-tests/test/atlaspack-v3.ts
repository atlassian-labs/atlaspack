import {join} from 'path';

import {AtlaspackV3, toFileSystemV3} from '@atlaspack/core';
import {NodePackageManager} from '@atlaspack/package-manager';
import {
  describe,
  fsFixture,
  inputFS,
  it,
  overlayFS,
} from '@atlaspack/test-utils';

describe('AtlaspackV3', function () {
  it('builds', async () => {
    // @ts-expect-error - TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
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

    let atlaspack = new AtlaspackV3({
      // @ts-expect-error - TS2345 - Argument of type '{ corePath: string; entries: string[]; fs: FileSystem; nodeWorkers: number; packageManager: PackageManager; }' is not assignable to parameter of type '{ fs?: unknown; nodeWorkers?: number | undefined; packageManager?: unknown; threads?: number | undefined; }'.
      corePath: '',
      entries: [join(__dirname, 'index.js')],
      fs: toFileSystemV3(overlayFS),
      nodeWorkers: 1,
      packageManager: new NodePackageManager(inputFS, __dirname),
    });

    await atlaspack.buildAssetGraph();
  });
});
