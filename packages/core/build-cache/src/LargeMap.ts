/**
 * A Map implementation that can exceed Node 24's Map size limit.
 *
 * LargeMap works around the Maximum maps size limit by using multiple internal Maps,
 * creating a new one when the current Map reaches the size limit.
 *
 * This is a minimal implementation supporting only has/get/set - intended as a
 * temporary solution until we no longer need large JS serialization
 */
export class LargeMap<K, V> {
  maps: Map<K, V>[];
  maxSize: number;

  constructor(maxSize: number = Math.pow(2, 23)) {
    this.maps = [new Map()];
    this.maxSize = maxSize;
  }

  set(key: K, value: V): this {
    // Update existing key if found
    if (this.maps.length > 1) {
      for (let map of this.maps) {
        if (map.has(key)) {
          map.set(key, value);
          return this;
        }
      }
    }

    // Otherwise, add to last map
    let lastMap = this.maps[this.maps.length - 1];
    if (lastMap.size >= this.maxSize) {
      lastMap = new Map();
      this.maps.push(lastMap);
    }
    lastMap.set(key, value);
    return this;
  }

  get(key: K): V | undefined {
    for (let map of this.maps) {
      if (map.has(key)) {
        return map.get(key);
      }
    }
    return undefined;
  }

  has(key: K): boolean {
    for (let map of this.maps) {
      if (map.has(key)) {
        return true;
      }
    }
    return false;
  }
}
