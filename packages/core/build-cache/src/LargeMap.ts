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
  singleMap: boolean;
  lastMap: Map<K, V>;

  constructor(maxSize: number = Math.pow(2, 23)) {
    this.lastMap = new Map();
    this.maps = [this.lastMap];
    this.singleMap = true;
    this.maxSize = maxSize;
  }

  set(key: K, value: V): this {
    // Update existing key if found
    if (!this.singleMap) {
      for (let map of this.maps) {
        if (map.has(key)) {
          map.set(key, value);
          return this;
        }
      }
    }

    // Otherwise, add to last map
    if (this.lastMap.size >= this.maxSize) {
      this.lastMap = new Map();
      this.maps.push(this.lastMap);
      this.singleMap = false;
    }

    this.lastMap.set(key, value);

    return this;
  }

  get(key: K): V | undefined {
    if (this.singleMap) {
      return this.lastMap.get(key);
    }

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
