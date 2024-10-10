// @flow

import type {Symbol as NapiSymbol} from '@atlaspack/rust';
import type {
  AssetSymbols as ClassicAssetSymbols,
  MutableAssetSymbols as ClassicMutableAssetSymbols,
  MutableDependencySymbols as ClassicMutableDependencySymbols,
  Symbol,
  SourceLocation,
  Meta,
} from '@atlaspack/types';

export class MutableDependencySymbols
  implements ClassicMutableDependencySymbols
{
  #symbols: Map<Symbol, DependencyAssetSymbol>;
  #locals: Set<Symbol>;

  get isCleared(): boolean {
    return this.#symbols.size === 0;
  }

  constructor(inner: NapiSymbol[]) {
    this.#symbols = new Map();
    this.#locals = new Set();
    for (const {exported, local, loc, isWeak} of inner) {
      this.set(exported, local, loc, isWeak);
    }
  }

  ensure(): void {
    throw new Error('MutableDependencySymbols.ensure()');
  }

  get(exportSymbol: Symbol): ?DependencyAssetSymbol {
    return this.#symbols.get(exportSymbol);
  }

  hasExportSymbol(exportSymbol: Symbol): boolean {
    return this.#symbols.has(exportSymbol);
  }

  hasLocalSymbol(local: Symbol): boolean {
    return this.#locals.has(local);
  }

  exportSymbols(): Iterable<Symbol> {
    return this.#symbols.keys();
  }

  set(
    exportSymbol: Symbol,
    local: Symbol,
    loc: ?SourceLocation,
    isWeak: ?boolean,
  ): void {
    this.#locals.add(local);
    this.#symbols.set(exportSymbol, {
      local,
      loc,
      meta: {},
      isWeak: isWeak ?? false,
    });
  }

  delete(exportSymbol: Symbol): void {
    let existing = this.#symbols.get(exportSymbol);
    if (!existing) return;
    this.#locals.delete(existing.local);
    this.#symbols.delete(exportSymbol);
  }

  /*::  @@iterator(): Iterator<[Symbol, DependencyAssetSymbol]> { return ({}: any); } */
  // $FlowFixMe Flow can't do computed properties
  [Symbol.iterator]() {
    return this.#symbols.values();
  }
}

// TODO
export class AssetSymbols
  extends Array<[Symbol, AssetSymbol]>
  implements ClassicAssetSymbols
{
  isCleared: boolean;

  // eslint-disable-next-line no-unused-vars
  get(exportSymbol: Symbol): ?AssetSymbol {}

  // eslint-disable-next-line no-unused-vars
  hasExportSymbol(exportSymbol: Symbol): boolean {
    throw new Error('AssetSymbols.hasExportSymbol');
  }

  // eslint-disable-next-line no-unused-vars
  hasLocalSymbol(local: Symbol): boolean {
    throw new Error('AssetSymbols.hasExportSymbol');
  }

  exportSymbols(): Iterable<Symbol> {
    return [];
  }
}

// TODO
export class MutableAssetSymbols
  extends AssetSymbols
  implements ClassicMutableAssetSymbols
{
  ensure(): void {}
  set(
    // eslint-disable-next-line no-unused-vars
    exportSymbol: Symbol,
    // eslint-disable-next-line no-unused-vars
    local: Symbol,
    // eslint-disable-next-line no-unused-vars
    loc: ?SourceLocation,
    // eslint-disable-next-line no-unused-vars
    meta?: ?Meta,
  ): void {}

  // eslint-disable-next-line no-unused-vars
  delete(exportSymbol: Symbol): void {}
}

export type AssetSymbol = {|local: Symbol, loc: ?SourceLocation, meta?: ?Meta|};
export type DependencyAssetSymbol = {|
  local: Symbol,
  loc: ?SourceLocation,
  meta?: ?Meta,
  isWeak: boolean,
|};
