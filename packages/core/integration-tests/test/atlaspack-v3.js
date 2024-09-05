// @flow

import assert from 'assert';
import {join} from 'path';

import {AtlaspackV3, toFileSystemV3} from '@atlaspack/core';
import {NodePackageManager} from '@atlaspack/package-manager';
import {
  describe,
  fsFixture,
  inputFS,
  it,
  overlayFS,
  bundle,
  run,
} from '@atlaspack/test-utils';

describe('AtlaspackV3', function () {
  it('builds', async () => {
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
      corePath: '',
      entries: [join(__dirname, 'index.js')],
      fs: toFileSystemV3(overlayFS),
      nodeWorkers: 1,
      packageManager: new NodePackageManager(inputFS, __dirname),
    });

    await atlaspack.buildAssetGraph();
  });

  it.only('should build with html entry', async function () {
    await fsFixture(overlayFS, __dirname)`
        index.html:
          <script src="./index.js" />

        index.js:
          output = "it's working";
      `;

    let b = await bundle(join(__dirname, 'index.html'), {
      inputFS: overlayFS,
      outputFS: inputFS,
    });

    let res = await run(
      b,
      {
        output: null,
      },
      {require: false},
    );
    assert(res.output, "it's working");
  });
});
