// @flow
import assert from 'assert';
import {
  shardUrlUnchecked,
  shardUrl,
  domainShardingKey,
  applyShardToDomain,
} from '../src/index.js';

describe('domain sharding helpers', () => {
  beforeEach(() => {
    // $FlowFixMe
    delete globalThis[domainShardingKey];
  });

  describe('applyShardToDomain', () => {
    it('should use the base domain for shard 0', () => {
      const testBundle =
        'https://bundle-shard.assets.example.com/assets/test-bundle.123abc.js';

      const result = applyShardToDomain(testBundle, 0);

      assert.equal(
        result,
        'https://bundle-shard.assets.example.com/assets/test-bundle.123abc.js',
      );
    });

    it('should use the 0 shard domain for shard 1', () => {
      const testBundle =
        'https://bundle-shard.assets.example.com/assets/test-bundle.123abc.js';

      const result = applyShardToDomain(testBundle, 1);

      assert.equal(
        result,
        'https://bundle-shard-0.assets.example.com/assets/test-bundle.123abc.js',
      );
    });

    it('should use the 2 shard domain for shard 3', () => {
      const testBundle =
        'https://bundle-shard.assets.example.com/assets/test-bundle.123abc.js';

      const result = applyShardToDomain(testBundle, 3);

      assert.equal(
        result,
        'https://bundle-shard-2.assets.example.com/assets/test-bundle.123abc.js',
      );
    });
  });

  describe('shardUrlUnchecked', () => {
    it('should shard a URL', () => {
      const testBundle =
        'https://bundle-shard.assets.example.com/assets/test-bundle.123abc.js';

      const result = shardUrlUnchecked(testBundle, 5);

      assert.equal(
        result,
        'https://bundle-shard-1.assets.example.com/assets/test-bundle.123abc.js',
      );
    });

    it('should include the original domain in the shar', () => {
      const testBundle =
        'https://bundle-shard.assets.example.com/assets/test-bundle.123abc.js';

      const result = shardUrlUnchecked(testBundle, 5);

      assert.equal(
        result,
        'https://bundle-shard-1.assets.example.com/assets/test-bundle.123abc.js',
      );
    });

    it('should re-shard a domain that has already been sharded', () => {
      const testBundle =
        'https://bundle-shard-0.assets.example.com/assets/TestBundle.1a2b3c.js';

      const result = shardUrlUnchecked(testBundle, 5);

      assert.equal(
        result,
        'https://bundle-shard-2.assets.example.com/assets/TestBundle.1a2b3c.js',
      );
    });

    it('should handle domains with no .', () => {
      const testBundle = 'http://localhost/assets/testBundle.123abc.js';

      const result = shardUrlUnchecked(testBundle, 5);

      assert.equal(result, 'http://localhost-2/assets/testBundle.123abc.js');
    });

    it('should handle domains with ports', () => {
      const testBundle = 'http://localhost:8081/assets/testBundle.123abc.js';

      const result = shardUrlUnchecked(testBundle, 5);

      assert.equal(
        result,
        'http://localhost-2:8081/assets/testBundle.123abc.js',
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
        'https://bundle-shard-1.assets.example.com/assets/test-bundle.123abc.js',
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
