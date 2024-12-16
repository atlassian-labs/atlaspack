// @flow
import assert from 'assert';

import {fsFixture, overlayFS, bundle} from '@atlaspack/test-utils';

// $FlowFixMe importing TypeScript
import {getShardedBundleURL} from '../src/helpers/bundle-url-shards';

const createErrorStack = (url) => {
  // This error stack is copied from a local dev, with a bunch
  // of lines trimmed off the end so it's not unnecessarily long
  return `
Error
    at Object.getShardedBundleURL (http://localhost:8081/main-bundle.1a2fa8b7.js:15688:29)
    at a7u9v.6d3ceb6ac67fea50 (${url}.1a2fa8b7.js:361466:46)
    at newRequire (http://localhost:8081/main-bundle.1a2fa8b7.js:71:24)
    at localRequire (http://localhost:8081/main-bundle.1a2fa8b7.js:84:35)
    at 7H8wc.react-intl-next (http://localhost:8081/main-bundle.1a2fa8b7.js:279746:28)
    at newRequire (http://localhost:8081/main-bundle.1a2fa8b7.js:71:24)
    at localRequire (http://localhost:8081/main-bundle.1a2fa8b7.js:84:35)
    at 1nL5S../manifest (http://localhost:8081/main-bundle.1a2fa8b7.js:279714:17)
    at newRequire (http://localhost:8081/main-bundle.1a2fa8b7.js:71:24)
    at localRequire (http://localhost:8081/main-bundle.1a2fa8b7.js:84:35)
`.trim();
};

describe('bundle-url-shards helper', () => {
  describe('getShardedBundleURL', () => {
    beforeEach(() => {
      // $FlowFixMe
      delete globalThis.__ATLASPACK_ENABLE_DOMAIN_SHARDS;
    });

    it('should shard a URL if the global variable is set', () => {
      const testBundle = 'test-bundle.123abc.js';

      // $FlowFixMe
      globalThis.__ATLASPACK_ENABLE_DOMAIN_SHARDS = true;

      const err = new Error();
      err.stack = createErrorStack(
        'https://bundle-shard.assets.example.com/assets/ParentBundle.cba321.js',
      );

      const result = getShardedBundleURL(testBundle, 5, err);

      assert.equal(result, 'https://bundle-shard-0.assets.example.com/assets/');
    });

    it('should re-shard a domain that has already been sharded', () => {
      const testBundle = 'TestBundle.1a2b3c.js';

      // $FlowFixMe
      globalThis.__ATLASPACK_ENABLE_DOMAIN_SHARDS = true;

      const err = new Error();
      err.stack = createErrorStack(
        'https://bundle-shard-1.assets.example.com/assets/ParentBundle.cba321.js',
      );

      const result = getShardedBundleURL(testBundle, 5, err);

      assert.equal(result, 'https://bundle-shard-4.assets.example.com/assets/');
    });

    it('should not add a shard if the cookie is not present', () => {
      const testBundle = 'UnshardedBundle.9z8x7y.js';

      const err = new Error();
      err.stack = createErrorStack(
        'https://bundle-unsharded.assets.example.com/assets/ParentBundle.cba321.js',
      );

      const result = getShardedBundleURL(testBundle, 5, err);

      assert.equal(
        result,
        'https://bundle-unsharded.assets.example.com/assets/',
      );
    });
  });

  describe('compiled into JS Runtime', () => {
    it('should insert all arguments into compiled output', async () => {
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
      if (!mainBundle) return assert(mainBundle);

      const code = await overlayFS.readFile(mainBundle.filePath, 'utf-8');
      assert.ok(
        code.includes(
          `require("e3ed71d88565db").getShardedBundleURL('b.8575baaf.js', ${maxShards}) + "b.8575baaf.js"`,
        ),
        'Expected generated code for getShardedBundleURL was not found',
      );
    });
  });
});
