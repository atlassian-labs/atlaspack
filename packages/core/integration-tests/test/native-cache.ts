import assert from 'assert';
import path from 'path';
import {
  describe,
  it,
  fsFixture,
  inputFS,
  bundler,
  run,
} from '@atlaspack/test-utils';

/**
 * Integration tests for the native (Rust) transformer caching mechanism.
 *
 * These tests verify that the memoization cache correctly:
 * - Caches transformer results between builds
 * - Invalidates cache when source files change
 * - Returns correct results from cache
 */
describe.v3('Native cache', function () {
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
    assert.equal(buildOne.nativeCacheStats.transformerPipelines?.hits, 0);
    assert.equal(buildOne.nativeCacheStats.transformerPipelines?.misses, 2);

    let output = await run(buildOne.bundleGraph);
    assert.equal(output.default, 'should not fail');

    let buildTwo = await instance.run();
    // Two hits: one for index.js, one for the esmodule-helpers.js
    assert.equal(buildTwo.nativeCacheStats.transformerPipelines?.hits, 2);
    assert.equal(buildTwo.nativeCacheStats.transformerPipelines?.misses, 0);

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
    assert.equal(
      buildOne.nativeCacheStats.transformerPipelines?.uncacheables,
      2,
    );

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
    assert.equal(buildOne.nativeCacheStats.transformerPipelines?.bailouts, 2);

    // Verify bailouts are tracked by transformer name
    const bailoutsByTransformer =
      buildOne.nativeCacheStats.transformerPipelines?.bailoutsByTransformer;
    assert.ok(bailoutsByTransformer, 'bailoutsByTransformer should be defined');
    const transformerBailouts =
      bailoutsByTransformer['./transformer-plugin-2.ts'];
    assert.equal(
      transformerBailouts,
      2,
      'transformer-plugin-2.ts should have 2 bailouts',
    );

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
    assert.equal(buildOne.nativeCacheStats.transformerPipelines?.bailouts, 2);

    // Verify bailouts are tracked by transformer name
    const bailoutsByTransformer =
      buildOne.nativeCacheStats.transformerPipelines?.bailoutsByTransformer;
    assert.ok(bailoutsByTransformer, 'bailoutsByTransformer should be defined');
    const transformerBailouts =
      bailoutsByTransformer['./transformer-plugin-3.ts'];
    assert.equal(
      transformerBailouts,
      2,
      'transformer-plugin-3.ts should have 2 bailouts',
    );

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
    assert.equal(buildOne.nativeCacheStats.transformerPipelines?.misses, 2);

    let output = await run(buildOne.bundleGraph);
    assert.equal(output.default, 'should not true');
  });

  it.v3('should not cache when disableCache is set in setup', async () => {
    await fsFixture(inputFS, dir)`
        package.json:
          {
            "name": "cache-money"
          }

        yarn.lock:

        index.js:
          export default 'should not fail';

        .parcelrc:
          {
            "extends": "@atlaspack/config-default",
            "transformers": {
              "*.js": ["./transformer-plugin-disable-cache.ts", "..."]
            }
          }

        transformer-plugin-disable-cache.ts:
          import { Transformer } from '@atlaspack/plugin';

          export default new Transformer({
            async setup() {
              return {
                config: { replaceValue: 'cache' },
                disableCache: true,
              };
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

    // Transformer explicitly disabled caching via disableCache: true
    assert.equal(
      buildOne.nativeCacheStats.transformerPipelines?.uncacheables,
      2,
    );

    let output = await run(buildOne.bundleGraph);
    assert.equal(output.default, 'should not cache');

    // Second build should also be uncacheable (no cache hits)
    let buildTwo = await instance.run();
    assert.equal(
      buildTwo.nativeCacheStats.transformerPipelines?.uncacheables,
      2,
    );
    assert.equal(buildTwo.nativeCacheStats.transformerPipelines?.hits, 0);
  });
});
