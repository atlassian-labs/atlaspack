import assert from 'assert';

import {fsFixture, overlayFS, bundle} from '@atlaspack/test-utils';

import {
  getShardedBundleURL,
  getBaseURL,
} from '../src/helpers/bundle-url-shards';

const testingCookieName = 'DOMAIN_SHARDING_TEST';

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
    it('should shard a URL if the cookie is present', () => {
      const testBundle = 'test-bundle.123abc.js';

      const err = new Error();
      err.stack = createErrorStack(
        'https://bundle-shard.assets.example.com/assets/ParentBundle.cba321.js',
      );

      const result = getShardedBundleURL(
        testBundle,
        testingCookieName,
        `${testingCookieName}=1`,
        5,
        err,
      );

      assert.equal(result, 'https://bundle-shard-0.assets.example.com/assets/');
    });

    it('should re-shard a domain that has already been sharded', () => {
      const testBundle = 'TestBundle.1a2b3c.js';

      const err = new Error();
      err.stack = createErrorStack(
        'https://bundle-shard-1.assets.example.com/assets/ParentBundle.cba321.js',
      );

      const result = getShardedBundleURL(
        testBundle,
        testingCookieName,
        `${testingCookieName}=1`,
        5,
        err,
      );

      assert.equal(result, 'https://bundle-shard-4.assets.example.com/assets/');
    });

    it('should not add a shard if the cookie is not present', () => {
      const testBundle = 'UnshardedBundle.9z8x7y.js';

      const err = new Error();
      err.stack = createErrorStack(
        'https://bundle-unsharded.assets.example.com/assets/ParentBundle.cba321.js',
      );

      const result = getShardedBundleURL(
        testBundle,
        testingCookieName,
        `some.other.cookie=1`,
        5,
        err,
      );

      assert.equal(
        result,
        'https://bundle-unsharded.assets.example.com/assets/',
      );
    });
  });

  describe('getBaseUrl', () => {
    it('should return the URL with the filename removed', () => {
      const testUrl =
        'https://bundle-shard-3.assets.example.com/assets/testBundle.123abc.js';

      assert.equal(
        getBaseURL(testUrl),
        'https://bundle-shard-3.assets.example.com/assets/',
      );
    });

    it('should handle domains with no .', () => {
      const testUrl = 'http://localhost/assets/testBundle.123abc.js';

      assert.equal(getBaseURL(testUrl), 'http://localhost/assets/');
    });

    it('should handle domains with ports', () => {
      const testUrl = 'http://localhost:8081/assets/testBundle.123abc.js';

      assert.equal(getBaseURL(testUrl), 'http://localhost:8081/assets/');
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
                "maxShards": ${maxShards},
                "cookieName": "${testingCookieName}"
              }
            }
          }

        src/index.js:
          async function fn() {
            const a = await import('./a.js');
            console.log('a', a);
          }
          fn();

        src/a.js:
          export const a = 'A';

        yarn.lock:
      `;

      const bundleGraph = await bundle('src/index.js', {inputFS: overlayFS});

      const mainBundle = bundleGraph
        .getBundles()
        .find((b) => b.name === 'index.js');

      const code = await overlayFS.readFile(mainBundle.filePath, 'utf-8');
      assert.ok(
        code.includes(
          `require("64e4a85cd0b8d551").getShardedBundleURL('1Acoq', '${testingCookieName}', document.cookie, ${maxShards}) + "a.771579fd.js"`,
        ),
      );
    });
  });
});
