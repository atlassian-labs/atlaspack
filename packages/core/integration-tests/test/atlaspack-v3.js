// @flow

import assert from 'assert';
import {join} from 'path';

import {AtlaspackV3, FileSystemV3} from '@atlaspack/core';
import {NodePackageManager} from '@atlaspack/package-manager';
import {
  describe,
  fsFixture,
  inputFS,
  it,
  overlayFS,
  outputFS,
  bundle,
} from '@atlaspack/test-utils';
import {LMDBLiteCache} from '@atlaspack/cache';

async function assertOutputIsIdentical(entry: string) {
  let bundlesV3 = await bundle(entry, {
    inputFS: overlayFS,
  }).then((b) => b.getBundles());

  let bundlesV2 = await bundle(entry, {
    inputFS: overlayFS,
    featureFlags: {
      atlaspackV3: false,
    },
  }).then((b) => b.getBundles());

  assert.equal(bundlesV3.length, bundlesV2.length);

  for (let i = 0; i < bundlesV2.length; i++) {
    let v2Code = await outputFS.readFile(bundlesV2[i].filePath, 'utf8');
    let v3Code = await outputFS.readFile(bundlesV3[i].filePath, 'utf8');

    assert.equal(v3Code, v2Code);
  }
}

describe.v3('AtlaspackV3', function () {
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
      fs: new FileSystemV3(overlayFS),
      nodeWorkers: 1,
      packageManager: new NodePackageManager(inputFS, __dirname),
      lmdb: new LMDBLiteCache('.parcel-cache').getNativeRef(),
    });

    await atlaspack.buildAssetGraph();
  });

  describe.only('should mirror V2 output', () => {
    it('with dynamic resolver code', async () => {
      await fsFixture(overlayFS, __dirname)`
        resolver-code
          index.js:
            import theGlobal from 'theGlobal';
            sideEffectNoop(theGlobal)
          index.html:
            <script type="module" src="./index.js" />
          package.json:
            {
              "alias": {
                "theGlobal": {
                  "global": "MY_GLOBAL"
                }
              }
            }
          yarn.lock:
      `;

      await assertOutputIsIdentical(
        join(__dirname, 'resolver-code/index.html'),
      );
    });

    it('with CSS modules', async () => {
      await fsFixture(overlayFS, __dirname)`
        css-modules
          css.module.css:
            .composed {
              background: pink;
            }
            .foo {
              composes: composed;
              color: white;
            }
          index.js:
            import {foo} from './css.module.css';
            sideEffectNoop(foo)
          index.html:
            <script type="module" src="./index.js" />
      `;

      await assertOutputIsIdentical(join(__dirname, 'css-modules/index.html'));
    });
  });
});
