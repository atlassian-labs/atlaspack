import type {BundleGraph, Dependency, Asset} from '@atlaspack/types';

// Note: This file intentionally avoids reading the asset graph from cache.
// The integration parity tests compare symbol metadata exposed via the BundleGraph API.

type NormalizedSymbol = {
  local: string;
  loc: unknown;
  isWeak?: boolean;
  meta?: unknown;
};

function normalizeSymbolsMap(
  symbols: Map<string, any> | null | undefined,
): Record<string, NormalizedSymbol> | null {
  if (symbols == null) return null;

  let out: Record<string, NormalizedSymbol> = {};
  for (let [exported, data] of symbols) {
    out[exported] = {
      local: String(data?.local),
      loc: data?.loc ?? null,
      ...(data?.isWeak != null ? {isWeak: Boolean(data.isWeak)} : null),
      ...(data?.meta != null ? {meta: data.meta} : null),
    };
  }
  return out;
}

type NormalizedUsedSymbol = {
  asset: string;
  symbol: string | null;
};

function normalizeUsedSymbolsUp(
  usedSymbolsUp: Map<string, any> | null | undefined,
): Record<string, NormalizedUsedSymbol> | null {
  if (usedSymbolsUp == null) return null;

  let out: Record<string, NormalizedUsedSymbol> = {};
  for (let [exported, resolved] of usedSymbolsUp) {
    // Skip '*' symbol as it's a namespace marker
    if (exported === '*') continue;
    out[exported] = {
      asset: String(resolved?.asset ?? ''),
      symbol: resolved?.symbol ?? null,
    };
  }
  return out;
}

// NOTE: When comparing via BundleGraph we generally only have access to the *final used symbol set*,
// not the full per-symbol resolution map. We normalize this set for parity checks.

export type NormalizedSymbolTrackerSnapshot = {
  // Keyed by normalized file path.
  assets: Record<
    string,
    {
      symbols: Record<string, NormalizedSymbol> | null;
    }
  >;
  // Keyed by `sourceAssetId|specifier` (or just `unknown|specifier` where missing).
  dependencies: Record<
    string,
    {
      symbols: Record<string, NormalizedSymbol> | null;
    }
  >;
};

/**
 * Extract a stable, JSON-serializable view of symbol-related data from an AssetGraph.
 *
 * This is meant for parity tests where two builds should produce equivalent symbol metadata.
 */
export function extractSymbolTrackerSnapshot(
  bundleGraph: BundleGraph,
): NormalizedSymbolTrackerSnapshot {
  let assets: NormalizedSymbolTrackerSnapshot['assets'] = {};
  let dependencies: NormalizedSymbolTrackerSnapshot['dependencies'] = {};

  bundleGraph.traverse((node) => {
    if (node.type === 'asset') {
      let asset = node.value as Asset;
      assets[String(asset.filePath)] = {
        symbols: normalizeSymbolsMap((asset as any).symbols),
      };
    }

    if (node.type === 'dependency') {
      let dep = node.value as Dependency;
      let depKey = `${String((dep as any).sourceAssetId ?? 'unknown')}|${String(
        dep.specifier,
      )}`;

      // Use usedSymbolsUp for comparison - this is what the symbol tracker produces
      // The node here is the internal DependencyNode which has usedSymbolsUp
      let usedSymbolsUp = (node as any).usedSymbolsUp as
        | Map<string, any>
        | undefined;

      dependencies[depKey] = {
        symbols: usedSymbolsUp ? normalizeUsedSymbolsUp(usedSymbolsUp) : null,
      };
    }
  });

  return {assets, dependencies};
}
