import {LMDBLiteCache} from '@atlaspack/cache';

export interface CacheStats {
  size: number;
  count: number;
  keySize: number;
  assetContentCount: number;
  assetContentSize: number;
  assetMapCount: number;
  assetMapSize: number;
}

/**
 * Aggregate data based on general cache usage, by reading all entries
 * in the cache.
 *
 * This can take a non-neglible amount of time on large caches, but
 * will still be a lot faster than the graph deserialization steps.
 */
export function getCacheStats(cache: LMDBLiteCache): CacheStats {
  const stats: CacheStats = {
    size: 0,
    count: 0,
    keySize: 0,
    assetContentCount: 0,
    assetContentSize: 0,
    assetMapCount: 0,
    assetMapSize: 0,
  };

  for (const key of cache.keys()) {
    const value = cache.getBlobSync(key);
    stats.size += value.length;
    stats.keySize += Buffer.from(key).length;
    stats.count++;
    if (key.endsWith(':content')) {
      stats.assetContentCount++;
      stats.assetContentSize += value.length;
    } else if (key.endsWith(':map')) {
      stats.assetMapCount++;
      stats.assetMapSize += value.length;
    }
  }

  return stats;
}
