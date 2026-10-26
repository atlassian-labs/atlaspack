"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.clearCaches = clearCaches;
exports.memoize = memoize;
function _manyKeysMap() {
  const data = _interopRequireDefault(require("many-keys-map"));
  _manyKeysMap = function () {
    return data;
  };
  return data;
}
function _interopRequireDefault(e) { return e && e.__esModule ? e : { default: e }; }
let caches = [];
function clearCaches() {
  for (let cache of caches) {
    cache.clear();
  }
}
function memoize(fn) {
  let cache = new (_manyKeysMap().default)();
  caches.push(cache);
  return function (...args) {
    // Navigate through the cache hierarchy
    let cached = cache.get(args);
    if (cached !== undefined) {
      // If the result is cached, return it
      return cached;
    }

    // Calculate the result and cache it
    // @ts-expect-error TS2683
    const result = fn.apply(this, args);
    cache.set(args, result);
    return result;
  };
}