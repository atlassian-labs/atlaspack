declare module 'lru-cache' {
  export default class LRUCache<K, V> {
    constructor(options?: any);
    set(key: K, value: V): void;
    get(key: K): V | undefined;
    has(key: K): boolean;
    delete(key: K): boolean;
    clear(): void;
    size: number;
    max?: number;
    maxAge?: number;
  }
}
