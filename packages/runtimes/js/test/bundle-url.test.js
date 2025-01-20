// @flow
import assert from 'assert';

import {getBaseURL, getBundleURL} from '../src/helpers/bundle-url';

const createErrorStack = (url) => {
  // This error stack is copied from a local dev, with a bunch
  // of lines trimmed off the end so it's not unnecessarily long
  return `
Error
    at Object.getBundleURL (http://localhost:8081/main-bundle.1a2fa8b7.js:15688:29)
    at Object.getBundleURLCached (http://localhost:8081/main-bundle.1a2fa8b7.js:15688:29)
    at a7u9v.6d3ceb6ac67fea50 (${url}:361466:46)
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

describe('getBundleURL', () => {
  it('should return the URL of the bundle without the file name', () => {
    const testUrl =
      'https://bundle-domain.assets.example.com/assets/testBundle.123abc.js';

    const errorStack = createErrorStack(testUrl);
    const error = new Error();
    error.stack = errorStack;

    const result = getBundleURL('test-asset', error);

    assert.equal(result, 'https://bundle-domain.assets.example.com/assets/');
  });
});
