// @flow

import type {
  BundleBehavior,
  DependencyPriority,
  SpecifierType,
} from '@atlaspack/types';

/// BitFlags is used to map number/string types from napi types
export class BitFlags<K, V> {
  #kv: {[key: K]: V};
  #vk: {[key: V]: K};

  constructor(source: {[key: K]: V}) {
    this.#kv = source;
    this.#vk = Object.fromEntries(
      // $FlowFixMe
      Object.entries(source).map((a) => a.reverse()),
    );
  }

  into(key: K): V {
    const found = this.#kv[key];
    if (found === undefined) {
      // $FlowFixMe
      throw new Error(`Invalid BundleBehavior(${key})`);
    }
    return found;
  }

  intoArray(keys: K[]): V[] {
    return keys.map((key) => this.into(key));
  }

  from(key: V): K {
    const found = this.#vk[key];
    if (found === undefined) {
      // $FlowFixMe
      throw new Error(`Invalid BundleBehavior(${key})`);
    }
    return found;
  }

  fromArray(keys: V[]): K[] {
    return keys.map((key) => this.from(key));
  }
}

export const bundleBehaviorMap: BitFlags<BundleBehavior, number> = new BitFlags(
  {
    inline: 0,
    isolated: 1,
  },
);

export const dependencyPriorityMap: BitFlags<DependencyPriority, number> =
  new BitFlags({
    sync: 0,
    parallel: 1,
    lazy: 2,
    conditional: 3,
  });

export const packageConditionsMap: BitFlags<string, number> = new BitFlags({
  import: 0,
  require: 1,
  module: 2,
  node: 3,
  browser: 4,
  worker: 5,
  worklet: 6,
  electron: 7,
  development: 8,
  production: 9,
  types: 10,
  default: 11,
  style: 12,
  sass: 13,
});

export const specifierTypeMap: BitFlags<SpecifierType, number> = new BitFlags({
  esm: 0,
  commonjs: 1,
  url: 2,
  custom: 3,
});
