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
  // @ts-expect-error TS2564
  #value: Asset;
  // @ts-expect-error TS2564
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

  
  hasExportSymbol(exportSymbol: ISymbol): boolean {
    return Boolean(this.#value.symbols?.has(exportSymbol));
  }

  
  hasLocalSymbol(local: ISymbol): boolean {
    if (this.#value.symbols == null) {
      return false;
    }
    for (let s of this.#value.symbols.values()) {
      if (local === s.local) return true;
    }
    return false;
  }

  
  get(exportSymbol: ISymbol):
    | {
        local: ISymbol;
        loc: SourceLocation | null | undefined;
        meta?: Meta | null | undefined;
      }
    | null
    | undefined {
    // @ts-expect-error TS2322
    return fromInternalAssetSymbol(
      this.#options.projectRoot,
      // @ts-expect-error TS2345
      this.#value.symbols?.get(exportSymbol),
    );
  }

  get isCleared(): boolean {
    return this.#value.symbols == null;
  }
  
  exportSymbols(): Iterable<ISymbol> {
    return this.#value.symbols?.keys() ?? [];
  }
  
  // @ts-expect-error TS2416
  [Symbol.iterator]() {
    return this.#value.symbols
      ? this.#value.symbols[Symbol.iterator]()
      : EMPTY_ITERATOR;
  }

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
  // @ts-expect-error TS2564
  #value: Asset;
  // @ts-expect-error TS2564
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

  
  hasExportSymbol(exportSymbol: ISymbol): boolean {
    return Boolean(this.#value.symbols?.has(exportSymbol));
  }

  
  hasLocalSymbol(local: ISymbol): boolean {
    if (this.#value.symbols == null) {
      return false;
    }
    for (let s of this.#value.symbols.values()) {
      if (local === s.local) return true;
    }
    return false;
  }

  
  get(exportSymbol: ISymbol):
    | {
        local: ISymbol;
        loc: SourceLocation | null | undefined;
        meta?: Meta | null | undefined;
      }
    | null
    | undefined {
    // @ts-expect-error TS2322
    return fromInternalAssetSymbol(
      this.#options.projectRoot,
      // @ts-expect-error TS2345
      this.#value.symbols?.get(exportSymbol),
    );
  }

  get isCleared(): boolean {
    return this.#value.symbols == null;
  }

  
  exportSymbols(): Iterable<ISymbol> {
    // @ts-expect-error TS2322
    return this.#value.symbols.keys();
  }
  
  // @ts-expect-error TS2416
  [Symbol.iterator]() {
    return this.#value.symbols
      ? this.#value.symbols[Symbol.iterator]()
      : EMPTY_ITERATOR;
  }

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

  
  set(
    exportSymbol: ISymbol,
    local: ISymbol,
    loc?: SourceLocation | null,
    meta?: Meta | null,
  ) {
    nullthrows(this.#value.symbols).set(exportSymbol, {
      local,
      loc: toInternalSourceLocation(this.#options.projectRoot, loc),
      meta,
    });
  }

  
  delete(exportSymbol: ISymbol) {
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
  // @ts-expect-error TS2564
  #value: Dependency;
  // @ts-expect-error TS2564
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

  hasExportSymbol(exportSymbol: ISymbol): boolean {
    // @ts-expect-error TS2345
    return Boolean(this.#value.symbols?.has(exportSymbol));
  }

  
  hasLocalSymbol(local: ISymbol): boolean {
    if (this.#value.symbols) {
      for (let s of this.#value.symbols.values()) {
        // @ts-expect-error TS2367
        if (local === s.local) return true;
      }
    }
    return false;
  }

  
  get(exportSymbol: ISymbol):
    | {
        local: ISymbol;
        loc: SourceLocation | null | undefined;
        isWeak: boolean;
        meta?: Meta | null | undefined;
      }
    | null
    | undefined {
    // @ts-expect-error TS2322
    return fromInternalDependencySymbol(
      this.#options.projectRoot,
      // @ts-expect-error TS2345
      nullthrows(this.#value.symbols).get(exportSymbol),
    );
  }

  get isCleared(): boolean {
    return this.#value.symbols == null;
  }

  exportSymbols(): Iterable<ISymbol> {
    // @ts-expect-error TS2322
    return this.#value.symbols ? this.#value.symbols.keys() : EMPTY_ITERABLE;
  }

  // @ts-expect-error TS2416
  [Symbol.iterator]() {
    return this.#value.symbols
      ? this.#value.symbols[Symbol.iterator]()
      : EMPTY_ITERATOR;
  }

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

  
  set(
    exportSymbol: ISymbol,
    local: ISymbol,
    loc?: SourceLocation | null,
    isWeak?: boolean | null,
  ) {
    let symbols = nullthrows(this.#value.symbols);
    // @ts-expect-error TS2345
    symbols.set(exportSymbol, {
      local,
      loc: toInternalSourceLocation(this.#options.projectRoot, loc),
      // @ts-expect-error TS2345
      isWeak: (symbols.get(exportSymbol)?.isWeak ?? true) && (isWeak ?? false),
    });
  }

  
  delete(exportSymbol: ISymbol) {
    // @ts-expect-error TS2345
    nullthrows(this.#value.symbols).delete(exportSymbol);
  }
}

function fromInternalAssetSymbol(
  projectRoot: string,
  value:
    | undefined
    | {
        // @ts-expect-error TS2304
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
        // @ts-expect-error TS2304
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
