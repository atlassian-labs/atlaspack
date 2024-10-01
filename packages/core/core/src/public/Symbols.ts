import type {
  Symbol as ISymbol,
  MutableAssetSymbols as IMutableAssetSymbols,
  AssetSymbols as IAssetSymbols,
  MutableDependencySymbols as IMutableDependencySymbols,
  SourceLocation,
  Meta,
} from '@atlaspack/types';
import type {Asset, Dependency, AtlaspackOptions} from '../types';

import nullthrows from 'nullthrows';
import {fromInternalSourceLocation, toInternalSourceLocation} from '../utils';

const EMPTY_ITERABLE = {
  [Symbol.iterator]() {
    return EMPTY_ITERATOR;
  },
} as const;

const EMPTY_ITERATOR = {
  next() {
    return {done: true};
  },
} as const;

const inspect = Symbol.for('nodejs.util.inspect.custom');

let valueToSymbols: WeakMap<Asset, AssetSymbols> = new WeakMap();
export class AssetSymbols implements IAssetSymbols {
  /*::
  @@iterator(): Iterator<[ISymbol, {|local: ISymbol, loc: ?SourceLocation, meta?: ?Meta|}]> { return ({}: any); }
  */
  // @ts-expect-error - TS2564 - Property '#value' has no initializer and is not definitely assigned in the constructor.
  #value: Asset;
  // @ts-expect-error - TS2564 - Property '#options' has no initializer and is not definitely assigned in the constructor.
  #options: AtlaspackOptions;

  constructor(options: AtlaspackOptions, asset: Asset) {
    let existing = valueToSymbols.get(asset);
    if (existing != null) {
      return existing;
    }

    this.#value = asset;
    this.#options = options;
    valueToSymbols.set(asset, this);
    return this;
  }

  // @ts-expect-error - TS2416 - Property 'hasExportSymbol' in type 'AssetSymbols' is not assignable to the same property in base type 'AssetSymbols'.
  hasExportSymbol(exportSymbol: ISymbol): boolean {
    // @ts-expect-error - TS2345 - Argument of type 'string' is not assignable to parameter of type 'symbol'.
    return Boolean(this.#value.symbols?.has(exportSymbol));
  }

  // @ts-expect-error - TS2416 - Property 'hasLocalSymbol' in type 'AssetSymbols' is not assignable to the same property in base type 'AssetSymbols'.
  hasLocalSymbol(local: ISymbol): boolean {
    if (this.#value.symbols == null) {
      return false;
    }
    for (let s of this.#value.symbols.values()) {
      // @ts-expect-error - TS2367 - This condition will always return 'false' since the types 'string' and 'symbol' have no overlap.
      if (local === s.local) return true;
    }
    return false;
  }

  // @ts-expect-error - TS2416 - Property 'get' in type 'AssetSymbols' is not assignable to the same property in base type 'AssetSymbols'.
  get(exportSymbol: ISymbol):
    | {
        local: ISymbol;
        loc: SourceLocation | null | undefined;
        meta?: Meta | null | undefined;
      }
    | null
    | undefined {
    // @ts-expect-error - TS2322 - Type '{ local: symbol; meta: JSONObject | null | undefined; loc: SourceLocation | null | undefined; } | undefined' is not assignable to type '{ local: string; loc: SourceLocation | null | undefined; meta?: JSONObject | null | undefined; } | null | undefined'.
    return fromInternalAssetSymbol(
      this.#options.projectRoot,
      // @ts-expect-error - TS2345 - Argument of type 'string' is not assignable to parameter of type 'symbol'.
      this.#value.symbols?.get(exportSymbol),
    );
  }

  get isCleared(): boolean {
    return this.#value.symbols == null;
  }

  // @ts-expect-error - TS2416 - Property 'exportSymbols' in type 'AssetSymbols' is not assignable to the same property in base type 'AssetSymbols'.
  exportSymbols(): Iterable<ISymbol> {
    // @ts-expect-error - TS2322 - Type 'never[] | IterableIterator<symbol>' is not assignable to type 'Iterable<string>'.
    return this.#value.symbols?.keys() ?? [];
  }
  // $FlowFixMe
  // @ts-expect-error - TS2416 - Property '[Symbol.iterator]' in type 'AssetSymbols' is not assignable to the same property in base type 'AssetSymbols'.
  [Symbol.iterator]() {
    return this.#value.symbols
      ? this.#value.symbols[Symbol.iterator]()
      : EMPTY_ITERATOR;
  }

  // $FlowFixMe
  [inspect]() {
    return `AssetSymbols(${
      this.#value.symbols
        ? [...this.#value.symbols]
            .map(([s, {local}]: [any, any]) => `${s}:${local}`)
            .join(', ')
        : null
    })`;
  }
}

let valueToMutableAssetSymbols: WeakMap<Asset, MutableAssetSymbols> =
  new WeakMap();
export class MutableAssetSymbols implements IMutableAssetSymbols {
  /*::
  @@iterator(): Iterator<[ISymbol, {|local: ISymbol, loc: ?SourceLocation, meta?: ?Meta|}]> { return ({}: any); }
  */
  // @ts-expect-error - TS2564 - Property '#value' has no initializer and is not definitely assigned in the constructor.
  #value: Asset;
  // @ts-expect-error - TS2564 - Property '#options' has no initializer and is not definitely assigned in the constructor.
  #options: AtlaspackOptions;

  constructor(options: AtlaspackOptions, asset: Asset) {
    let existing = valueToMutableAssetSymbols.get(asset);
    if (existing != null) {
      return existing;
    }
    this.#value = asset;
    this.#options = options;
    return this;
  }

  // immutable

  // @ts-expect-error - TS2416 - Property 'hasExportSymbol' in type 'MutableAssetSymbols' is not assignable to the same property in base type 'MutableAssetSymbols'.
  hasExportSymbol(exportSymbol: ISymbol): boolean {
    // @ts-expect-error - TS2345 - Argument of type 'string' is not assignable to parameter of type 'symbol'.
    return Boolean(this.#value.symbols?.has(exportSymbol));
  }

  // @ts-expect-error - TS2416 - Property 'hasLocalSymbol' in type 'MutableAssetSymbols' is not assignable to the same property in base type 'MutableAssetSymbols'.
  hasLocalSymbol(local: ISymbol): boolean {
    if (this.#value.symbols == null) {
      return false;
    }
    for (let s of this.#value.symbols.values()) {
      // @ts-expect-error - TS2367 - This condition will always return 'false' since the types 'string' and 'symbol' have no overlap.
      if (local === s.local) return true;
    }
    return false;
  }

  // @ts-expect-error - TS2416 - Property 'get' in type 'MutableAssetSymbols' is not assignable to the same property in base type 'MutableAssetSymbols'.
  get(exportSymbol: ISymbol):
    | {
        local: ISymbol;
        loc: SourceLocation | null | undefined;
        meta?: Meta | null | undefined;
      }
    | null
    | undefined {
    // @ts-expect-error - TS2322 - Type '{ local: symbol; meta: JSONObject | null | undefined; loc: SourceLocation | null | undefined; } | undefined' is not assignable to type '{ local: string; loc: SourceLocation | null | undefined; meta?: JSONObject | null | undefined; } | null | undefined'.
    return fromInternalAssetSymbol(
      this.#options.projectRoot,
      // @ts-expect-error - TS2345 - Argument of type 'string' is not assignable to parameter of type 'symbol'.
      this.#value.symbols?.get(exportSymbol),
    );
  }

  get isCleared(): boolean {
    return this.#value.symbols == null;
  }

  // @ts-expect-error - TS2416 - Property 'exportSymbols' in type 'MutableAssetSymbols' is not assignable to the same property in base type 'MutableAssetSymbols'.
  exportSymbols(): Iterable<ISymbol> {
    // @ts-expect-error - TS2322 - Type 'IterableIterator<symbol>' is not assignable to type 'Iterable<string>'. | TS2533 - Object is possibly 'null' or 'undefined'.
    return this.#value.symbols.keys();
  }
  // $FlowFixMe
  // @ts-expect-error - TS2416 - Property '[Symbol.iterator]' in type 'MutableAssetSymbols' is not assignable to the same property in base type 'MutableAssetSymbols'.
  [Symbol.iterator]() {
    return this.#value.symbols
      ? this.#value.symbols[Symbol.iterator]()
      : EMPTY_ITERATOR;
  }

  // $FlowFixMe
  [inspect]() {
    return `MutableAssetSymbols(${
      this.#value.symbols
        ? [...this.#value.symbols]
            .map(([s, {local}]: [any, any]) => `${s}:${local}`)
            .join(', ')
        : null
    })`;
  }

  // mutating

  ensure(): void {
    if (this.#value.symbols == null) {
      this.#value.symbols = new Map();
    }
  }

  // @ts-expect-error - TS2416 - Property 'set' in type 'MutableAssetSymbols' is not assignable to the same property in base type 'MutableAssetSymbols'.
  set(
    exportSymbol: ISymbol,
    local: ISymbol,
    loc?: SourceLocation | null,
    meta?: Meta | null,
  ) {
    // @ts-expect-error - TS2345 - Argument of type 'string' is not assignable to parameter of type 'symbol'.
    nullthrows(this.#value.symbols).set(exportSymbol, {
      local,
      loc: toInternalSourceLocation(this.#options.projectRoot, loc),
      meta,
    });
  }

  // @ts-expect-error - TS2416 - Property 'delete' in type 'MutableAssetSymbols' is not assignable to the same property in base type 'MutableAssetSymbols'.
  delete(exportSymbol: ISymbol) {
    // @ts-expect-error - TS2345 - Argument of type 'string' is not assignable to parameter of type 'symbol'.
    nullthrows(this.#value.symbols).delete(exportSymbol);
  }
}

let valueToMutableDependencySymbols: WeakMap<
  Dependency,
  MutableDependencySymbols
> = new WeakMap();
export class MutableDependencySymbols implements IMutableDependencySymbols {
  /*::
  @@iterator(): Iterator<[ISymbol, {|local: ISymbol, loc: ?SourceLocation, isWeak: boolean, meta?: ?Meta|}]> { return ({}: any); }
  */
  // @ts-expect-error - TS2564 - Property '#value' has no initializer and is not definitely assigned in the constructor.
  #value: Dependency;
  // @ts-expect-error - TS2564 - Property '#options' has no initializer and is not definitely assigned in the constructor.
  #options: AtlaspackOptions;

  constructor(options: AtlaspackOptions, dep: Dependency) {
    let existing = valueToMutableDependencySymbols.get(dep);
    if (existing != null) {
      return existing;
    }
    this.#value = dep;
    this.#options = options;
    return this;
  }

  // immutable:

  // @ts-expect-error - TS2416 - Property 'hasExportSymbol' in type 'MutableDependencySymbols' is not assignable to the same property in base type 'MutableDependencySymbols'.
  hasExportSymbol(exportSymbol: ISymbol): boolean {
    // @ts-expect-error - TS2345 - Argument of type 'string' is not assignable to parameter of type 'symbol'.
    return Boolean(this.#value.symbols?.has(exportSymbol));
  }

  // @ts-expect-error - TS2416 - Property 'hasLocalSymbol' in type 'MutableDependencySymbols' is not assignable to the same property in base type 'MutableDependencySymbols'.
  hasLocalSymbol(local: ISymbol): boolean {
    if (this.#value.symbols) {
      for (let s of this.#value.symbols.values()) {
        // @ts-expect-error - TS2367 - This condition will always return 'false' since the types 'string' and 'symbol' have no overlap.
        if (local === s.local) return true;
      }
    }
    return false;
  }

  // @ts-expect-error - TS2416 - Property 'get' in type 'MutableDependencySymbols' is not assignable to the same property in base type 'MutableDependencySymbols'.
  get(exportSymbol: ISymbol):
    | {
        local: ISymbol;
        loc: SourceLocation | null | undefined;
        isWeak: boolean;
        meta?: Meta | null | undefined;
      }
    | null
    | undefined {
    // @ts-expect-error - TS2322 - Type '{ local: symbol; meta: JSONObject | null | undefined; isWeak: boolean; loc: SourceLocation | null | undefined; } | undefined' is not assignable to type '{ local: string; loc: SourceLocation | null | undefined; isWeak: boolean; meta?: JSONObject | null | undefined; } | null | undefined'.
    return fromInternalDependencySymbol(
      this.#options.projectRoot,
      // @ts-expect-error - TS2345 - Argument of type 'string' is not assignable to parameter of type 'symbol'.
      nullthrows(this.#value.symbols).get(exportSymbol),
    );
  }

  get isCleared(): boolean {
    return this.#value.symbols == null;
  }

  // @ts-expect-error - TS2416 - Property 'exportSymbols' in type 'MutableDependencySymbols' is not assignable to the same property in base type 'MutableDependencySymbols'.
  exportSymbols(): Iterable<ISymbol> {
    // @ts-expect-error - TS2322 - Type '{ readonly [Symbol.iterator]: () => { readonly next: () => { done: boolean; }; }; } | IterableIterator<symbol>' is not assignable to type 'Iterable<string>'.
    return this.#value.symbols ? this.#value.symbols.keys() : EMPTY_ITERABLE;
  }

  // $FlowFixMe
  // @ts-expect-error - TS2416 - Property '[Symbol.iterator]' in type 'MutableDependencySymbols' is not assignable to the same property in base type 'MutableDependencySymbols'.
  [Symbol.iterator]() {
    return this.#value.symbols
      ? this.#value.symbols[Symbol.iterator]()
      : EMPTY_ITERATOR;
  }

  // $FlowFixMe
  [inspect]() {
    return `MutableDependencySymbols(${
      this.#value.symbols
        ? [...this.#value.symbols]
            .map(
              ([s, {local, isWeak}]: [any, any]) =>
                `${s}:${local}${isWeak ? '?' : ''}`,
            )
            .join(', ')
        : null
    })`;
  }

  // mutating:

  ensure(): void {
    if (this.#value.symbols == null) {
      this.#value.symbols = new Map();
    }
  }

  // @ts-expect-error - TS2416 - Property 'set' in type 'MutableDependencySymbols' is not assignable to the same property in base type 'MutableDependencySymbols'.
  set(
    exportSymbol: ISymbol,
    local: ISymbol,
    loc?: SourceLocation | null,
    isWeak?: boolean | null,
  ) {
    let symbols = nullthrows(this.#value.symbols);
    // @ts-expect-error - TS2345 - Argument of type 'string' is not assignable to parameter of type 'symbol'.
    symbols.set(exportSymbol, {
      local,
      loc: toInternalSourceLocation(this.#options.projectRoot, loc),
      // @ts-expect-error - TS2345 - Argument of type 'string' is not assignable to parameter of type 'symbol'.
      isWeak: (symbols.get(exportSymbol)?.isWeak ?? true) && (isWeak ?? false),
    });
  }

  // @ts-expect-error - TS2416 - Property 'delete' in type 'MutableDependencySymbols' is not assignable to the same property in base type 'MutableDependencySymbols'.
  delete(exportSymbol: ISymbol) {
    // @ts-expect-error - TS2345 - Argument of type 'string' is not assignable to parameter of type 'symbol'.
    nullthrows(this.#value.symbols).delete(exportSymbol);
  }
}

function fromInternalAssetSymbol(
  projectRoot: string,
  value:
    | undefined
    | {
        loc: InternalSourceLocation | null | undefined;
        local: symbol;
        meta?: Meta | null | undefined;
      },
) {
  return (
    value && {
      local: value.local,
      meta: value.meta,
      loc: fromInternalSourceLocation(projectRoot, value.loc),
    }
  );
}

function fromInternalDependencySymbol(
  projectRoot: string,
  value:
    | undefined
    | {
        isWeak: boolean;
        loc: InternalSourceLocation | null | undefined;
        local: symbol;
        meta?: Meta | null | undefined;
      },
) {
  return (
    value && {
      local: value.local,
      meta: value.meta,
      isWeak: value.isWeak,
      loc: fromInternalSourceLocation(projectRoot, value.loc),
    }
  );
}
