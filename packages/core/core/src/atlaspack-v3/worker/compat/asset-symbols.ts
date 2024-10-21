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
  #symbols: Map<symbol, DependencyAssetSymbol>;
  #locals: Set<symbol>;

  get isCleared(): boolean {
    return this.#symbols.size === 0;
  }

  constructor(inner: NapiSymbol[] | null | undefined | null) {
    this.#symbols = new Map();
    this.#locals = new Set();
    for (const {exported, local, loc, isWeak} of inner || []) {
      this.set(exported, local, loc, isWeak);
    }
  }

  ensure(): void {
    throw new Error('MutableDependencySymbols.ensure()');
  }

  get(exportSymbol: symbol): DependencyAssetSymbol | null | undefined {
    return this.#symbols.get(exportSymbol);
  }

  hasExportSymbol(exportSymbol: symbol): boolean {
    return this.#symbols.has(exportSymbol);
  }

  hasLocalSymbol(local: symbol): boolean {
    return this.#locals.has(local);
  }

  exportSymbols(): Iterable<symbol> {
    return this.#symbols.keys();
  }

  set(
    exportSymbol: symbol,
    local: symbol,
    loc?: SourceLocation | null,
    isWeak?: boolean | null,
  ): void {
    this.#locals.add(local);
    this.#symbols.set(exportSymbol, {
      local,
      loc,
      meta: {},
      isWeak: isWeak ?? false,
    });
  }

  delete(exportSymbol: symbol): void {
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
  #symbols: Map<symbol, AssetSymbol>;
  #locals: Set<symbol>;

  get isCleared(): boolean {
    return this.#symbols.size === 0;
  }

  constructor(inner: NapiSymbol[] | null | undefined | null) {
    this.#symbols = new Map();
    this.#locals = new Set();
    for (const {exported, local, loc} of inner || []) {
      this.#set(exported, local, loc);
    }
  }

  #set(exportSymbol: symbol, local: symbol, loc?: SourceLocation | null): undefined {
    this.#locals.add(local);
    this.#symbols.set(exportSymbol, {
      local,
      loc,
      meta: {},
    });
  }

  get(exportSymbol: symbol): AssetSymbol | null | undefined {
    return this.#symbols.get(exportSymbol);
  }

  hasExportSymbol(exportSymbol: symbol): boolean {
    return this.#symbols.has(exportSymbol);
  }

  hasLocalSymbol(local: symbol): boolean {
    return this.#locals.has(local);
  }

  exportSymbols(): Iterable<symbol> {
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
  #symbols: Map<symbol, AssetSymbol>;
  #locals: Set<symbol>;

  get isCleared(): boolean {
    return this.#symbols.size === 0;
  }

  constructor(inner: NapiSymbol[] | null | undefined | null) {
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

  set(exportSymbol: symbol, local: symbol, loc?: SourceLocation | null): void {
    this.#locals.add(local);
    this.#symbols.set(exportSymbol, {
      local,
      loc,
      meta: {},
    });
  }

  get(exportSymbol: symbol): AssetSymbol | null | undefined {
    return this.#symbols.get(exportSymbol);
  }

  hasExportSymbol(exportSymbol: symbol): boolean {
    return this.#symbols.has(exportSymbol);
  }

  hasLocalSymbol(local: symbol): boolean {
    return this.#locals.has(local);
  }

  exportSymbols(): Iterable<symbol> {
    return this.#symbols.keys();
  }

  delete(exportSymbol: symbol): void {
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

export type AssetSymbol = {
  local: symbol,
  loc: SourceLocation | null | undefined,
  meta?: Meta | null | undefined
};
export type DependencyAssetSymbol = {
  local: symbol,
  loc: SourceLocation | null | undefined,
  meta?: Meta | null | undefined,
  isWeak: boolean
};
