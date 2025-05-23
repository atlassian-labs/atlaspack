// @flow strict-local

type Entry<K, V> = {|
  next: Entry<K, V> | null,
  prev: Entry<K, V> | null,
  key: K,
  value: V,
|};

export class LRUCache<K, V> {
  #map: Map<K, Entry<K, V>>;
  #lru: Entry<K, V> | null;
  #maxSize: number;

  constructor(maxSize: number) {
    this.#maxSize = maxSize;
    this.#map = new Map();
    this.#lru = null;
  }

  has(key: K): boolean {
    return this.#map.has(key);
  }

  get(key: K): V | null {
    const entry = this.#map.get(key);
    if (entry) {
      this.updateLRU(entry);
    }
    return entry?.value ?? null;
  }

  set(key: K, value: V): void {
    const entry = {key, value, next: null, prev: null};
    this.#map.set(key, entry);
    this.updateLRU(entry);
    this.evict();
  }

  getLRU(): K | null {
    return this.#lru?.key ?? null;
  }

  updateLRU(entry: Entry<K, V>): void {
    if (entry.prev) {
      entry.prev.next = entry.next;
    }
    if (entry.next) {
      entry.next.prev = entry.prev;
    }

    entry.prev = null;
    if (this.#lru) {
      this.#lru.prev = entry;
    }
    entry.next = this.#lru;
    this.#lru = entry;
  }

  evict(): void {
    if (this.#map.size > this.#maxSize) {
      let current: Entry<K, V> | null = this.#lru;
      while (current != null) {
        current = current.next;
      }

      if (current) {
        this.#map.delete(current.key);
        if (current.next) {
          current.next.prev = current.prev;
        }
        if (current.prev) {
          current.prev.next = null;
        }
        current.prev = null;
        current.next = null;
      }
    }
  }
}
