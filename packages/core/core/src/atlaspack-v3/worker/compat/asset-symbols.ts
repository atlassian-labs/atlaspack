import type {Symbol as NapiSymbol} from '@atlaspack/rust';
import type {
  AssetSymbols as IAssetSymbols,
  MutableAssetSymbols as IMutableAssetSymbols,
  MutableDependencySymbols as IMutableDependencySymbols,
  Symbol,
  SourceLocation,
  Meta,
} from '@atlaspack/types';

export class MutableDependencySymbols implements IMutableDependencySymbols {
  #symbols: Map<Symbol, DependencyAssetSymbol>;
  #locals: Set<Symbol>;

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

  get(exportSymbol: Symbol): DependencyAssetSymbol | null | undefined {
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

  delete(exportSymbol: Symbol): void {
    let existing = this.#symbols.get(exportSymbol);
    if (!existing) return;
    this.#locals.delete(existing.local);
    this.#symbols.delete(exportSymbol);
  }

  /*::  @@iterator(): Iterator<[Symbol, DependencyAssetSymbol]> { return ({}: any); } */
  // @ts-expect-error TS2416
  [Symbol.iterator]() {
    return this.#symbols.values();
  }
}

export class AssetSymbols implements IAssetSymbols {
  #symbols: Map<Symbol, AssetSymbol>;
  #locals: Set<Symbol>;

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

  #set(
    exportSymbol: Symbol,
    local: Symbol,
    loc?: SourceLocation | null,
  ): undefined {
    this.#locals.add(local);
    this.#symbols.set(exportSymbol, {
      local,
      loc,
      meta: {},
    });
  }

  get(exportSymbol: Symbol): AssetSymbol | null | undefined {
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
  // @ts-expect-error TS2416
  [Symbol.iterator]() {
    return this.#symbols.values();
  }
}

export class MutableAssetSymbols implements IMutableAssetSymbols {
  #symbols: Map<Symbol, AssetSymbol>;
  #locals: Set<Symbol>;

  get isCleared(): boolean {
    return this.#symbols.size === 0;
  }

  constructor(inner: NapiSymbol[] | null | undefined | null) {
    this.#symbols = new Map();
    this.#locals = new Set();
    for (const {
      exported,
      loc,
      local,
      isEsmExport,
      selfReferenced,
      isStaticBindingSafe,
    } of inner || []) {
      this.set(exported, local, loc, {
        isEsm: isEsmExport,
        isStaticBindingSafe,
        selfReferenced,
      });
    }
  }

  intoNapi(): NapiSymbol[] {
    const results: NapiSymbol[] = [];
    for (const [exportSymbol, {local, loc, meta}] of this.#symbols.entries()) {
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
        isEsmExport: Boolean(meta?.isEsm),
        isStaticBindingSafe: Boolean(meta?.isStaticBindingSafe),
        isWeak: false,
        selfReferenced: Boolean(meta?.selfReferenced),
      });
    }
    return results;
  }

  ensure(): void {
    throw new Error('MutableAssetSymbols.ensure()');
  }

  set(
    exportSymbol: Symbol,
    local: Symbol,
    loc?: SourceLocation | null,
    meta?: Meta | null,
  ): void {
    this.#locals.add(local);
    this.#symbols.set(exportSymbol, {
      local,
      loc,
      meta,
    });
  }

  get(exportSymbol: Symbol): AssetSymbol | null | undefined {
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
  // @ts-expect-error TS2416
  [Symbol.iterator]() {
    return this.#symbols.values();
  }
}

export type AssetSymbol = {
  local: Symbol;
  loc: SourceLocation | null | undefined;
  meta?: Meta | null | undefined;
};

export type DependencyAssetSymbol = {
  local: Symbol;
  loc: SourceLocation | null | undefined;
  meta?: Meta | null | undefined;
  isWeak: boolean;
};
