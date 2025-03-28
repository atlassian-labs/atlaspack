// @flow

const buildCaches: Array<Map<any, any>> = [];

class AntiCache extends Map {
  set() {}
}

export function createBuildCache<K, V>(): Map<K, V> {
  let cache = new Map<K, V>();
  buildCaches.push(cache);
  return new AntiCache();
}

export function clearBuildCaches() {
  for (let cache of buildCaches) {
    cache.clear();
  }
}
