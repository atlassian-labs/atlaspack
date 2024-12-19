// @flow
import assert from 'assert';
import path from 'path';

import {fsFixture, overlayFS, bundle, run} from '@atlaspack/test-utils';
import {domainShardingKey} from '@atlaspack/domain-sharding';

const maxShards = 8;

describe('domain-sharding', () => {
  describe('should insert all arguments into compiled JS Runtime output', () => {
    it('for bundle manifest urls', async () => {
      await fsFixture(overlayFS)`
          package.json:
            {
              "name": "bundle-sharding-test",
              "targets": {
                "default": {
                  "context": "browser",
                  "optimize": false
                }
              },
              "@atlaspack/runtime-js": {
                "domainSharding": {
                  "maxShards": ${maxShards}
                }
              }
            }

          src/index.js:
            async function fn() {
              const a = await import('./a.js');
              const b = await import('./b.js');
              console.log('a', a, b);
            }
            fn();

          src/a.js:
            export const a = async () => {
              const b = await import('./b');
              return b + 'A';
            }
          src/b.js:
            export const b = 'B';

          yarn.lock:
        `;

      const bundleGraph = await bundle('src/index.js', {
        inputFS: overlayFS,
        mode: 'production',
      });

      const mainBundle = bundleGraph
        .getBundles()
        .find((b) => b.name === 'index.js');
      if (!mainBundle) return assert(mainBundle);

      const code = await overlayFS.readFile(mainBundle.filePath, 'utf-8');

      assert.ok(
        code.includes(
          `require("85e3bc75ab94a411")(require("f41955f5cc01151").shardUrl(require("40cc202a4c7abf8d").resolve("aVRxe"), ${maxShards}))`,
          'Expected generated code for shardUrl was not found',
        ),
      );
    });

    it('for all other urls', async () => {
      const maxShards = 8;
      await fsFixture(overlayFS)`
        package.json:
          {
            "name": "bundle-sharding-test",
            "@atlaspack/runtime-js": {
              "domainSharding": {
                "maxShards": ${maxShards}
              }
            }
          }

        src/index.js:
          async function fn() {
            const a = await import('./a.js');
            const b = await import('./b.js');
            console.log('a', a, b);
          }
          fn();

        src/a.js:
          export const a = async () => {
            const b = await import('./b');
            return b + 'A';
          }
        src/b.js:
          export const b = 'B';

        yarn.lock:
      `;

      const bundleGraph = await bundle('src/index.js', {inputFS: overlayFS});

      const mainBundle = bundleGraph
        .getBundles()
        .find((b) => b.name === 'index.js');
      const commonBundle = bundleGraph
        .getBundles()
        .find((b) => b.displayName === 'b.[hash].js');

      assert(commonBundle, 'Unable to locate the shared bundle');

      const commonFileName = path.basename(commonBundle?.filePath ?? '');

      if (!mainBundle) return assert(mainBundle);

      const code = await overlayFS.readFile(mainBundle.filePath, 'utf-8');
      assert.ok(
        code.includes(
          `require("e480067c5bab431e")(require("5091f5df3a0c51b6").shardUrl(require("5e2c91749d676db2").getBundleURL('d8wEr') + "${commonFileName}", ${maxShards})`,
        ),
        'Expected generated code for shardUrl was not found',
      );
    });

    it('for ESM loaded bundle manifest', async () => {
      const maxShards = 8;
      await fsFixture(overlayFS)`
        package.json:
          {
            "name": "bundle-sharding-test",
            "@atlaspack/runtime-js": {
              "domainSharding": {
                "maxShards": ${maxShards}
              }
            }
          }

        src/index.js:
          async function fn() {
            const a = await import('./a.js');
            const b = await import('./b.js');
            console.log('a', a, b);
          }
          fn();

        src/a.js:
          export const a = async () => {
            const b = await import('./b');
            return b + 'A';
          }
        src/b.js:
          export const b = 'B';

        yarn.lock:
      `;

      const bundleGraph = await bundle('src/index.js', {
        inputFS: overlayFS,
        mode: 'production',
        defaultTargetOptions: {
          shouldOptimize: false,
          outputFormat: 'esmodule',
        },
      });

      const mainBundle = bundleGraph
        .getBundles()
        .find((b) => b.name === 'index.js');

      if (!mainBundle) {
        return assert(mainBundle);
      }

      // $FlowFixMe - Flow doesn't seem to know about doesNotReject
      await assert.doesNotReject(
        run(bundleGraph, {[domainShardingKey]: true}),
        'Expected bundle to be able to execute',
      );

      const code = await overlayFS.readFile(mainBundle.filePath, 'utf-8');

      const expectedCode = `var$load = (parcelRequire("cfSxn"))(${maxShards});`
        .replaceAll('$', '\\$')
        .replaceAll('(', '\\(')
        .replaceAll(')', '\\)');

      assert.match(code, new RegExp(expectedCode));
    });
  });
});
