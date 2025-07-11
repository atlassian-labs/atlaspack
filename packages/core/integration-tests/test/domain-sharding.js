// @flow
import assert from 'assert';
import path from 'path';

import {fsFixture, overlayFS, bundle, run} from '@atlaspack/test-utils';
import {domainShardingKey} from '@atlaspack/domain-sharding';
import nullthrows from 'nullthrows';

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
              sideEffectNoop('a', a, b);
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
      const re =
        /require\((.*?)\)\(require\((.*?)\)\.shardUrl\(require\((.*?)\)\.resolve\((.*?)\),(.*?)\)\)/gm;
      const matches = Array.from(code.matchAll(re));

      assert(matches.length > 0);
      assert.equal(parseInt(matches[0][5].trim(), 10), maxShards);
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
            sideEffectNoop('a', a, b);
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
      const re =
        /require\((.*?)\)\(require\((.*?)\)\.shardUrl\(require\((.*?)\)\.getBundleURL\((.*?)\).*?\+(.*?),(.*?)\)/gm;
      const matches = Array.from(code.matchAll(re));
      assert(matches.length > 0);
      assert.equal(
        matches[1][5].trim().replaceAll('"', '').replaceAll("'", ''),
        commonFileName,
      );
      assert.equal(parseInt(matches[1][6].trim(), 10), maxShards);
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
            sideEffectNoop('a', a, b);
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
      const esmLoadAsset = nullthrows(
        bundleGraph.traverse((node, _, actions) => {
          if (
            node.type === 'asset' &&
            node.value.filePath.includes('helpers/browser/esm-js-loader-shards')
          ) {
            actions.stop();
            return node.value;
          }
        }),
        'Could not find esm-js-loader-shard asset',
      );

      const expectedCode =
        `var$load = (parcelRequire("${bundleGraph.getAssetPublicId(
          esmLoadAsset,
        )}"))(${maxShards});`
          .replaceAll('$', '\\$')
          .replaceAll('(', '\\(')
          .replaceAll(')', '\\)');

      assert.match(code, new RegExp(expectedCode));
    });
  });
});
