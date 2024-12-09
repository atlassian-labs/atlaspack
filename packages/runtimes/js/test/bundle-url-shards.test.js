import assert from 'assert';

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

describe.only('bundle-url-shards helper', () => {
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
});
