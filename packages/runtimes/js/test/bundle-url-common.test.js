// @flow
import assert from 'assert';

// $FlowFixMe importing TypeScript
import {getBaseURL} from '../src/helpers/bundle-url-common';

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
