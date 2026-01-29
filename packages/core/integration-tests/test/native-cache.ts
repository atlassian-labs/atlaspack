import assert from 'assert';
import path from 'path';
import {
  describe,
  it,
  fsFixture,
  inputFS,
  bundler,
  run,
  disableV2,
} from '@atlaspack/test-utils';

/**
 * Integration tests for the native (Rust) transformer caching mechanism.
 *
 * These tests verify that the memoization cache correctly:
 * - Caches transformer results between builds
 * - Invalidates cache when source files change
 * - Returns correct results from cache
 */
describe('Native cache', function () {
  disableV2();

  let dir: string;

  beforeEach(async function () {
    dir = path.join(__dirname, 'native-cache-fixture');
    await inputFS.rimraf(dir);
    await inputFS.mkdirp(dir);
  });

  afterEach(async function () {
    await inputFS.rimraf(dir);
  });

  it('should cache with stats', async () => {
    await fsFixture(inputFS, dir)`
        index.js:
          export default 'should not fail';
      `;

    let instance = bundler(path.join(dir, 'index.js'), {
      inputFS,
      featureFlags: {
        v3Caching: true,
      },
    });

    let buildOne = await instance.run();
    assert.equal(buildOne.nativeCacheStats.hits, 0);
    assert.equal(buildOne.nativeCacheStats.misses, 2);

    let output = await run(buildOne.bundleGraph);
    assert.equal(output.default, 'should not fail');

    let buildTwo = await instance.run();
    // Two hits: one for index.js, one for the esmodule-helpers.js
    assert.equal(buildTwo.nativeCacheStats.hits, 2);
    assert.equal(buildTwo.nativeCacheStats.misses, 0);

    output = await run(buildTwo.bundleGraph);
    assert.equal(output.default, 'should not fail');
  });

  it('should not cache when old Transformer API is used', async () => {
    await fsFixture(inputFS, dir)`
        package.json:
          {
            "name": "cache-n-stuff"
          }

        yarn.lock:

        index.js:
          export default 'should not fail';

        .parcelrc:
          {
            "extends": "@atlaspack/config-default",
            "transformers": {
              "*.js": ["./transformer-plugin.ts", "..."]
            }
          }

        transformer-plugin.ts:
          import { Transformer } from '@atlaspack/plugin';

          export default new Transformer({
            async loadConfig() {
              return {replaceValue: 'cache'};
            },
            async transform({ asset, config }) {
              const code = await asset.getCode();
              asset.setCode(code.replace('fail', config.replaceValue));
              return [asset];
            }
          });

      `;

    let instance = bundler(path.join(dir, 'index.js'), {
      inputFS,
      featureFlags: {
        v3Caching: true,
      },
    });

    let buildOne = await instance.run();
    assert.equal(buildOne.nativeCacheStats.uncacheables, 2);

    let output = await run(buildOne.bundleGraph);
    assert.equal(output.default, 'should not cache');
  });

  it('should not cache when unreported env usage is detected', async () => {
    await fsFixture(inputFS, dir)`
        package.json:
          {
            "name": "cache-n-stuff"
          }

        yarn.lock:

        index.js:
          export default 'should not fail';

        .parcelrc:
          {
            "extends": "@atlaspack/config-default",
            "transformers": {
              "*.js": ["./transformer-plugin-2.ts", "..."]
            }
          }

        transformer-plugin-2.ts:
          import { Transformer } from '@atlaspack/plugin';

          export default new Transformer({
            async setup() {
              return { config: {} }
            },
            async transform({ asset }) {
              const code = await asset.getCode();
              asset.setCode(code.replace('fail', process.env.ATLASPACK_V3));
              return [asset];
            }
          });

      `;

    let instance = bundler(path.join(dir, 'index.js'), {
      inputFS,
      featureFlags: {
        v3Caching: true,
      },
    });

    let buildOne = await instance.run();
    assert.equal(buildOne.nativeCacheStats.bailouts, 2);

    let output = await run(buildOne.bundleGraph);
    assert.equal(output.default, 'should not true');
  });

  it('should not cache when unreported fs usage is detected', async () => {
    await fsFixture(inputFS, dir)`
        package.json:
          {
            "name": "cache-n-stuff"
          }

        yarn.lock:

        index.js:
          export default 'should not fail';

        .parcelrc:
          {
            "extends": "@atlaspack/config-default",
            "transformers": {
              "*.js": ["./transformer-plugin-3.ts", "..."]
            }
          }

        transformer-plugin-3.ts:
          import { Transformer } from '@atlaspack/plugin';
          import fs from 'fs';

          export default new Transformer({
            async setup() {
              return { config: {} }
            },
            async transform({ asset }) {
              const code = await asset.getCode();
              asset.setCode(code.replace('fail', fs.readFileSync('${dir}/replace.txt', 'utf8')));
              return [asset];
            }
          });

        replace.txt:
          cache
      `;

    let instance = bundler(path.join(dir, 'index.js'), {
      inputFS,
      featureFlags: {
        v3Caching: true,
      },
    });

    let buildOne = await instance.run();
    assert.equal(buildOne.nativeCacheStats.bailouts, 2);

    let output = await run(buildOne.bundleGraph);
    assert.equal(output.default, 'should not cache');
  });

  it('should cache when reported env usage is detected', async () => {
    await fsFixture(inputFS, dir)`
        package.json:
          {
            "name": "cache-n-stuff"
          }

        yarn.lock:

        index.js:
          export default 'should not fail';

        .parcelrc:
          {
            "extends": "@atlaspack/config-default",
            "transformers": {
              "*.js": ["./transformer-plugin-4.ts", "..."]
            }
          }

        transformer-plugin-4.ts:
          import { Transformer } from '@atlaspack/plugin';

          export default new Transformer({
            async setup() {
              return { config: {}, env: ['ATLASPACK_V3'] }
            },
            async transform({ asset }) {
              const code = await asset.getCode();
              asset.setCode(code.replace('fail', process.env.ATLASPACK_V3));
              return [asset];
            }
          });

      `;

    let instance = bundler(path.join(dir, 'index.js'), {
      inputFS,
      featureFlags: {
        v3Caching: true,
      },
    });

    let buildOne = await instance.run();
    assert.equal(buildOne.nativeCacheStats.misses, 2);

    let output = await run(buildOne.bundleGraph);
    assert.equal(output.default, 'should not true');
  });
});
