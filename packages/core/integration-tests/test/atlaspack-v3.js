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
import type {InitialAtlaspackOptions} from '@atlaspack/types';

async function assertOutputIsIdentical(
  entry: string,
  options?: InitialAtlaspackOptions,
) {
  let bundlesV3 = await bundle(entry, {
    ...options,
    inputFS: overlayFS,
  }).then((b) => b.getBundles());

  let bundlesV2 = await bundle(entry, {
    ...options,
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

  describe('should mirror V2 output', () => {
    it('with scope hoisting enabled', async () => {
      await fsFixture(overlayFS, __dirname)`
        scope-hoist
          node_modules/library/named.js:
            export default function namedFunction(arg) {
              return arg;
            }
          node_modules/library/index.js:
            import namedFunction from './named.js';
            export {namedFunction};
          node_modules/library/package.json:
            {"sideEffects": false}
          index.js:
            import {namedFunction} from 'library';
            sideEffectNoop(namedFunction(''));
          index.html:
            <script type="module" src="./index.js" />
      `;

      await assertOutputIsIdentical(join(__dirname, 'scope-hoist/index.html'), {
        defaultTargetOptions: {
          shouldScopeHoist: true,
        },
      });
    });

    it('with Assets that change type', async () => {
      await fsFixture(overlayFS, __dirname)`
        type-change
          name.json:
            {"name": "fred"}
          index.js:
            import * as json from './name.json';
            sideEffectNoop(json.name);
          index.html:
            <script type="module" src="./index.js" />
      `;

      await assertOutputIsIdentical(join(__dirname, 'type-change/index.html'));
    });

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
