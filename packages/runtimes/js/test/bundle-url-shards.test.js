import {
  getShardedBundleURL,
  getBaseURL,
} from '../src/helpers/bundle-url-shards';

const testingCookieName = 'DOMAIN_SHARDING_TEST';

describe('getShardedBundleURL', () => {
  it('should shard a URL if the cookie is present', () => {
    const testBundle = 'TestBundle.123abc.js';

    // TODO: mock the error stacktrace somehow

    const result = getShardedBundleURL(
      testBundle,
      testingCookieName,
      `${testingCookieName}=1`,
      5,
    );

    expect(result).toBe('https://bundle-shard-3.assets.example.com');
  });

  it('should re-shard a domain that has already been sharded', () => {
    const testBundle = 'TestBundle.123abc.js';

    // TODO: mock the error stacktrace somehow

    const result = getShardedBundleURL(
      testBundle,
      testingCookieName,
      `${testingCookieName}=1`,
      5,
    );

    expect(result).toBe('https://bundle-shard-3.assets.example.com');
  });
});

describe('getBaseUrl', () => {
  it('should return the URL with the filename removed', () => {
    const testUrl =
      'https://bundle-shard-3.assets.example.com/assets/testBundle.123abc.js';

    expect(getBaseURL(testUrl)).toBe(
      'https://bundle-shard-3.assets.example.com/assets/',
    );
  });

  it('should handle domains with no .', () => {
    const testUrl = 'http://localhost/assets/testBundle.123abc.js';

    expect(getBaseURL(testUrl)).toBe('http://localhost/assets/');
  });

  it('should handle domains with ports', () => {
    const testUrl = 'http://localhost:8081/assets/testBundle.123abc.js';

    expect(getBaseURL(testUrl)).toBe('http://localhost:8081/assets/');
  });
});
