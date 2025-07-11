// eslint-disable-next-line @typescript-eslint/no-explicit-any
const buildCaches: Array<Map<any, any>> = [];

export function createBuildCache<K, V>(): Map<K, V> {
  const cache = new Map<K, V>();
  buildCaches.push(cache);
  return cache;
}

export function clearBuildCaches() {
  for (const cache of buildCaches) {
    cache.clear();
  }
}
