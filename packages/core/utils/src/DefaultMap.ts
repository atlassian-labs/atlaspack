export class DefaultMap<K, V> extends Map<K, V> {
  _getDefault: (arg1: K) => V;

  constructor(getDefault: (arg1: K) => V, entries?: Iterable<[K, V]>) {
    // @ts-expect-error - TS2769 - No overload matches this call.
    super(entries);
    this._getDefault = getDefault;
  }

  get(key: K): V {
    let ret;
    if (this.has(key)) {
      ret = super.get(key);
    } else {
      ret = this._getDefault(key);
      this.set(key, ret);
    }

    // @ts-expect-error - TS2322 - Type 'V | undefined' is not assignable to type 'V'.
    return ret;
  }
}

interface IKey {}

// Duplicated from DefaultMap implementation for Flow
// Roughly mirrors https://github.com/facebook/flow/blob/2eb5a78d92c167117ba9caae070afd2b9f598599/lib/core.js#L617
export class DefaultWeakMap<K extends IKey, V> extends WeakMap<K, V> {
  _getDefault: (arg1: K) => V;

  constructor(getDefault: (arg1: K) => V, entries?: Iterable<[K, V]>) {
    // @ts-expect-error - TS2769 - No overload matches this call.
    super(entries);
    this._getDefault = getDefault;
  }

  get(key: K): V {
    let ret;
    if (this.has(key)) {
      ret = super.get(key);
    } else {
      ret = this._getDefault(key);
      this.set(key, ret);
    }

    // @ts-expect-error - TS2322 - Type 'V | undefined' is not assignable to type 'V'.
    return ret;
  }
}
