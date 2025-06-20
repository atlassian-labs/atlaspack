// @flow strict

// $FlowFixMe
import ManyKeysMap from 'many-keys-map';

let caches = [];

export function clearCaches() {
  for (let cache of caches) {
    cache.clear();
  }
}

export function memoize<Args: Array<mixed>, Return>(
  fn: (...args: Args) => Return,
): (...args: Args) => Return {
  let cache = new ManyKeysMap();
  caches.push(cache);

  return function (...args: Args): Return {
    // Navigate through the cache hierarchy
    let cached = cache.get(args);
    if (cached !== undefined) {
      // If the result is cached, return it
      return cached;
    }

    // Calculate the result and cache it
    const result = fn.apply(this, args);
    // $FlowFixMe
    cache.set(args, result);

    return result;
  };
}
