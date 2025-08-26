import type {Asset} from '@atlaspack/types';

/**
 * Tracks symbols declared in different scopes to detect and prevent duplicates.
 * This is used by the ScopeHoistingPackager to ensure that hoisted parcelRequire
 * variables don't conflict when multiple assets in the same scope reference the
 * same dependency.
 */
export class SymbolTracker {
  // Maps scope identifier to set of declared symbols in that scope
  private scopeSymbols: Map<string, Set<string>> = new Map();

  // Maps scope identifier to set of assets that belong to that scope
  private scopeAssets: Map<string, Set<string>> = new Map();

  // Maps asset ID to its scope identifier
  private assetToScope: Map<string, string> = new Map();

  /**
   * Determines the scope identifier for an asset.
   * Assets that are wrapped get their own scope, while hoisted assets
   * share the top-level scope or their parent's scope.
   */
  private getScopeId(
    asset: Asset,
    isWrapped: boolean,
    parentAsset?: Asset,
  ): string {
    if (isWrapped) {
      // Wrapped assets get their own scope
      return `wrapped:${asset.id}`;
    }

    if (parentAsset) {
      // Hoisted assets inherit their parent's scope
      const parentScope = this.assetToScope.get(parentAsset.id);
      if (parentScope) {
        return parentScope;
      }
    }

    // Default to top-level scope
    return 'top-level';
  }

  /**
   * Registers an asset in a scope.
   */
  registerAsset(asset: Asset, isWrapped: boolean, parentAsset?: Asset): void {
    const scopeId = this.getScopeId(asset, isWrapped, parentAsset);
    this.assetToScope.set(asset.id, scopeId);

    if (!this.scopeAssets.has(scopeId)) {
      this.scopeAssets.set(scopeId, new Set());
    }
    this.scopeAssets.get(scopeId)!.add(asset.id);

    if (!this.scopeSymbols.has(scopeId)) {
      this.scopeSymbols.set(scopeId, new Set());
    }
  }

  /**
   * Checks if a symbol is already declared in the given asset's scope.
   */
  isSymbolDeclared(asset: Asset, symbol: string): boolean {
    const scopeId = this.assetToScope.get(asset.id);
    if (!scopeId) {
      return false;
    }

    const scopeSymbols = this.scopeSymbols.get(scopeId);
    return scopeSymbols ? scopeSymbols.has(symbol) : false;
  }

  /**
   * Declares a symbol in the given asset's scope.
   * Returns true if the symbol was successfully declared, false if it was already declared.
   */
  declareSymbol(asset: Asset, symbol: string): boolean {
    const scopeId = this.assetToScope.get(asset.id);
    if (!scopeId) {
      return false;
    }

    const scopeSymbols = this.scopeSymbols.get(scopeId);
    if (!scopeSymbols) {
      return false;
    }

    if (scopeSymbols.has(symbol)) {
      return false; // Already declared
    }

    scopeSymbols.add(symbol);
    return true;
  }

  /**
   * Gets all symbols declared in the same scope as the given asset.
   */
  getScopeSymbols(asset: Asset): Set<string> {
    const scopeId = this.assetToScope.get(asset.id);
    if (!scopeId) {
      return new Set();
    }

    return new Set(this.scopeSymbols.get(scopeId) || []);
  }

  /**
   * Gets all assets in the same scope as the given asset.
   */
  getScopeAssets(asset: Asset): Set<string> {
    const scopeId = this.assetToScope.get(asset.id);
    if (!scopeId) {
      return new Set();
    }

    return new Set(this.scopeAssets.get(scopeId) || []);
  }

  /**
   * Gets the scope identifier for an asset.
   */
  getAssetScope(asset: Asset): string | undefined {
    return this.assetToScope.get(asset.id);
  }

  /**
   * Clears all tracking data.
   */
  clear(): void {
    this.scopeSymbols.clear();
    this.scopeAssets.clear();
    this.assetToScope.clear();
  }
}
