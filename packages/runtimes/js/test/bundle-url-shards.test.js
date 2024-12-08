import assert from 'assert';

import {
  getShardedBundleURL,
  getBaseURL,
} from '../src/helpers/bundle-url-shards';

const testingCookieName = 'DOMAIN_SHARDING_TEST';

const createErrorStack = (url) => {
  // This error stack is copied from a local dev
  return `
Error
    at getShardedBundleURL (http://localhost:8081/main-entry-bundle.1a2fa8b7.js:13663:29)
    at Object.getShardedBundleURLCached [as getShardedBundleURL] (http://localhost:8081/main-entry-bundle.1a2fa8b7.js:13639:17)
    at jrxWb.d7fe7af8a40b5d68 (${url}?1733635254967:695:74)
    at newRequire (http://localhost-4:8081/Heartbeat.bc74c821.js?1733635254967:71:24)
    at localRequire (http://localhost-4:8081/Heartbeat.bc74c821.js?1733635254967:84:35)
    at loader (http://localhost-4:8081/Heartbeat.bc74c821.js?1733635254967:691:29)
    at runLoader (http://localhost:8081/dashboard-spa-container-jquery3.40388a2a.js:56535:20)
    at Object.run (http://localhost:8081/dashboard-spa-container-jquery3.40388a2a.js:56211:28)
    at TaskManager.<anonymous> (http://localhost:8081/dashboard-spa-container-jquery3.40388a2a.js:56132:33)
    at invokeFunc (http://localhost:8081/dashboard-spa-container-jquery3.40388a2a.js:53665:23)
    at trailingEdge (http://localhost:8081/dashboard-spa-container-jquery3.40388a2a.js:53697:42)
    at timerExpired (http://localhost:8081/dashboard-spa-container-jquery3.40388a2a.js:53689:40)
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

      assert.equal(getBaseURL(testUrl), 'http://localhost-1/assets/');
    });

    it('should handle domains with ports', () => {
      const testUrl = 'http://localhost:8081/assets/testBundle.123abc.js';

      assert.equal(getBaseURL(testUrl), 'http://localhost-1:8081/assets/');
    });
  });
});
