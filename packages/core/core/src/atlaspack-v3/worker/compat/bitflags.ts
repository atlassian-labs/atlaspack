import type {
  BundleBehavior,
  DependencyPriority,
  SpecifierType,
} from '@atlaspack/types';

/// BitFlags is used to map number/string types from napi types
export class BitFlags<K> {
  // @ts-expect-error TS2344
  #kv: Partial<Record<K, number>>;
  #vk: Partial<Record<number, K>>;

  // @ts-expect-error TS2344
  constructor(source: Partial<Record<K, number>>) {
    this.#kv = source;
    this.#vk = Object.fromEntries(
      Object.entries(source).map((a) => a.reverse()),
    );
  }

  into(key: K): number {
    const found = this.#kv[key];
    if (found === undefined) {
      throw new Error(`Invalid BundleBehavior(${key})`);
    }
    return found;
  }

  intoNullable(key?: K | null): number | null | undefined {
    if (key === undefined || key === null) {
      return undefined;
    }
    return this.into(key);
  }

  from(key: number): K {
    const found = this.#vk[key];
    if (found === undefined) {
      throw new Error(`Invalid BundleBehavior(${key})`);
    }
    return found;
  }

  fromNullable(key?: number | null): K | null | undefined {
    if (key === undefined || key === null) {
      return undefined;
    }
    return this.from(key);
  }

  toArray(keys: number): K[] {
    let values = [];
    for (let [key, value] of Object.entries(this.#kv) as [K, number][]) {
      if ((keys & value) !== 0) {
        values.push(key);
      }
    }

    return values;
  }
}

export const bundleBehaviorMap: BitFlags<BundleBehavior> = new BitFlags({
  inline: 0,
  isolated: 1,
  inlineIsolated: 2,
});

export const dependencyPriorityMap: BitFlags<DependencyPriority> = new BitFlags(
  {
    sync: 0,
    parallel: 1,
    lazy: 2,
    conditional: 3,
  },
);

// Note: The bitflags must match the bitflags in the Rust code.
// crates/atlaspack_core/src/types/package_json.rs
export const packageConditionsMap: BitFlags<string> = new BitFlags({
  import: 1 << 0,
  require: 1 << 1,
  module: 1 << 2,
  node: 1 << 3,
  browser: 1 << 4,
  worker: 1 << 5,
  worklet: 1 << 6,
  electron: 1 << 7,
  development: 1 << 8,
  production: 1 << 9,
  types: 1 << 10,
  default: 1 << 11,
  style: 1 << 12,
  sass: 1 << 13,
});

export const specifierTypeMap: BitFlags<SpecifierType> = new BitFlags({
  esm: 0,
  commonjs: 1,
  url: 2,
  custom: 3,
});
