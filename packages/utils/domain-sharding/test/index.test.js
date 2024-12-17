// @flow
import assert from 'assert';
import {shardUrlUnchecked, shardUrl, domainShardingKey} from '../src/index.js';

describe('domain sharding helpers', () => {
  beforeEach(() => {
    // $FlowFixMe
    delete globalThis[domainShardingKey];
  });

  describe('shardUrlUnchecked', () => {
    it('should shard a URL', () => {
      const testBundle =
        'https://bundle-shard.assets.example.com/assets/test-bundle.123abc.js';

      const result = shardUrlUnchecked(testBundle, 5);

      assert.equal(result, 'https://bundle-shard-0.assets.example.com/assets/');
    });

    it('should re-shard a domain that has already been sharded', () => {
      const testBundle =
        'https://bundle-shard-0.assets.example.com/assets/TestBundle.1a2b3c.js';

      const result = shardUrlUnchecked(testBundle, 5);

      assert.equal(result, 'https://bundle-shard-4.assets.example.com/assets/');
    });

    it('should handle domains with no .', () => {
      const testBundle = 'http://localhost/assets/testBundle.123abc.js';

      const result = shardUrlUnchecked(testBundle, 5);

      assert.equal(result, 'http://localhost-3/assets/testBundle.123abc.js');
    });

    it('should handle domains with ports', () => {
      const testBundle = 'http://localhost:8081/assets/testBundle.123abc.js';

      const result = shardUrlUnchecked(testBundle, 5);

      assert.equal(
        result,
        'http://localhost-3:8081/assets/testBundle.123abc.js',
      );
    });
  });

  describe('shardUrl', () => {
    it('should add a shard if the window property is set', () => {
      globalThis[domainShardingKey] = true;
      const testBundle =
        'https://bundle-shard.assets.example.com/assets/test-bundle.123abc.js';

      const result = shardUrl(testBundle, 5);

      assert.equal(
        result,
        'https://bundle-shard-0.assets.example.com/assets/test-bundle.123abc.js',
      );
    });

    it('should not add a shard if the window property is not set', () => {
      const testBundle =
        'https://bundle-shard.assets.example.com/assets/test-bundle.123abc.js';

      const result = shardUrl(testBundle, 5);

      assert.equal(result, testBundle);
    });
  });
});
