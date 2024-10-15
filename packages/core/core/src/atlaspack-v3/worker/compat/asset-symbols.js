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

  constructor(inner: ?(NapiSymbol[]) | null) {
    this.#symbols = new Map();
    this.#locals = new Set();
    for (const {exported, local, loc, isWeak} of inner || []) {
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

export class AssetSymbols implements ClassicAssetSymbols {
  #symbols: Map<Symbol, AssetSymbol>;
  #locals: Set<Symbol>;

  get isCleared(): boolean {
    return this.#symbols.size === 0;
  }

  constructor(inner: ?(NapiSymbol[]) | null) {
    this.#symbols = new Map();
    this.#locals = new Set();
    for (const {exported, local, loc} of inner || []) {
      this.#set(exported, local, loc);
    }
  }

  #set(exportSymbol: Symbol, local: Symbol, loc: ?SourceLocation): void {
    this.#locals.add(local);
    this.#symbols.set(exportSymbol, {
      local,
      loc,
      meta: {},
    });
  }

  get(exportSymbol: Symbol): ?AssetSymbol {
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

  intoNapi(): NapiSymbol[] {
    return [];
  }

  /*::  @@iterator(): Iterator<[Symbol, AssetSymbol]> { return ({}: any); } */
  // $FlowFixMe Flow can't do computed properties
  [Symbol.iterator]() {
    return this.#symbols.values();
  }
}

export class MutableAssetSymbols implements ClassicMutableAssetSymbols {
  #symbols: Map<Symbol, AssetSymbol>;
  #locals: Set<Symbol>;

  get isCleared(): boolean {
    return this.#symbols.size === 0;
  }

  constructor(inner: ?(NapiSymbol[]) | null) {
    this.#symbols = new Map();
    this.#locals = new Set();
    for (const {exported, local, loc} of inner || []) {
      this.set(exported, local, loc);
    }
  }

  intoNapi(): NapiSymbol[] {
    const results: NapiSymbol[] = [];
    for (const [exportSymbol, {local, loc}] of this.#symbols.entries()) {
      results.push({
        exported: exportSymbol,
        local,
        loc: loc
          ? {
              filePath: loc.filePath,
              start: {
                line: loc.start.line,
                column: loc.start.column,
              },
              end: {
                line: loc.end.line,
                column: loc.end.column,
              },
            }
          : undefined,
        // TODO
        isWeak: false,
        isEsmExport: false,
        selfReferenced: false,
      });
    }
    return results;
  }

  ensure(): void {
    throw new Error('MutableAssetSymbols.ensure()');
  }

  set(exportSymbol: Symbol, local: Symbol, loc: ?SourceLocation): void {
    this.#locals.add(local);
    this.#symbols.set(exportSymbol, {
      local,
      loc,
      meta: {},
    });
  }

  get(exportSymbol: Symbol): ?AssetSymbol {
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

  delete(exportSymbol: Symbol): void {
    let existing = this.#symbols.get(exportSymbol);
    if (!existing) return;
    this.#locals.delete(existing.local);
    this.#symbols.delete(exportSymbol);
  }

  /*::  @@iterator(): Iterator<[Symbol, AssetSymbol]> { return ({}: any); } */
  // $FlowFixMe Flow can't do computed properties
  [Symbol.iterator]() {
    return this.#symbols.values();
  }
}

export type AssetSymbol = {|local: Symbol, loc: ?SourceLocation, meta?: ?Meta|};
export type DependencyAssetSymbol = {|
  local: Symbol,
  loc: ?SourceLocation,
  meta?: ?Meta,
  isWeak: boolean,
|};
