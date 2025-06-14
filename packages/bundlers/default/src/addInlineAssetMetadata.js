// @flow strict-local
import type {MutableBundleGraph, Asset, Dependency} from '@atlaspack/types';

import nullthrows from 'nullthrows';
import path from 'path';

interface ScopeHoist {
  from: Asset;
  to: Asset;
}

/**
 * Add `asset.meta.inline = true` for all assets which have only one incomingDependency until
 * no more assets have one incomingDependency.
 *
 * Adds `dep.meta.duplicate = true` to dependencies which are duplicated after an asset has been
 * inlined.
 */
export function addInlineAssetMetadata(assetGraph: MutableBundleGraph): void {
  const incomingDependencyMap = new Map();
  const scopeHoistQueue: ScopeHoist[] = [];

  function hasReExport(dependency: Dependency) {
    for (const [, {local}] of dependency.symbols) {
      if (local.includes('re_export')) {
        return true;
      }
    }
    return false;
  }

  function queueAssetIfOneIncomingDependency(
    asset: Asset,
    incomingDependencies: Dependency[],
  ) {
    if (!asset.meta.shouldWrap && incomingDependencies.length === 1) {
      const [dependency] = incomingDependencies;
      if (dependency.priority !== 'lazy') {
        const assetToScopeHoistInto =
          assetGraph.getAssetWithDependency(dependency);
        // TODO: When is this undefined?
        if (assetToScopeHoistInto) {
          const isCircularDependency = assetGraph
            .getIncomingDependencies(assetToScopeHoistInto)
            .some((dep) => dep.sourceAssetId === asset.id);
          if (
            dependency.sourceAssetType === 'js' &&
            !isCircularDependency &&
            !hasReExport(dependency)
          ) {
            scopeHoistQueue.push({
              from: asset,
              to: assetToScopeHoistInto,
            });
          }
        }
      }
    }
  }

  function getSyncResolvedAssetDependencies(asset: Asset) {
    const resolvedAssets: Map<Asset, Dependency> = new Map();

    let resolvedAsset;
    for (const dependency of assetGraph.getDependencies(asset)) {
      if (
        dependency.priority !== 'lazy' &&
        (resolvedAsset = assetGraph.getResolvedAsset(dependency))
      ) {
        resolvedAssets.set(resolvedAsset, dependency);
      }
    }
    return resolvedAssets;
  }

  assetGraph.traverse(({type, value}) => {
    if (type === 'asset') {
      const incomingDependencies = assetGraph.getIncomingDependencies(value);
      incomingDependencyMap.set(value, incomingDependencies);
      queueAssetIfOneIncomingDependency(value, incomingDependencies);
    }
  }, null);

  let scopeHoist, from, to;
  while ((scopeHoist = scopeHoistQueue.pop())) {
    ({from, to} = scopeHoist);
    from.meta.inline = to.id;

    const assetToScopeHoistAssetDependencies =
      getSyncResolvedAssetDependencies(from);
    const assetToScopeHoistIntoAssetDependencies =
      getSyncResolvedAssetDependencies(to);

    /**
     * All assets which are used in both `from` and `to` should have their
     * incomingDependencies updated.
     */
    let isPastScopeHoist = false;
    for (const [asset, dep] of assetToScopeHoistIntoAssetDependencies) {
      if (asset.id === from.id) {
        isPastScopeHoist = true;
        continue;
      }

      const depToScopeHoist = assetToScopeHoistAssetDependencies.get(asset);

      // Asset is not contained in the dep being scope hoisted (`from`)
      if (!depToScopeHoist) {
        continue;
      }

      /** dep from assetToScopeHoistInto has the duplicate if it's not an inline require */
      if (isPastScopeHoist) {
        if (!depToScopeHoist.meta.shouldWrap) {
          dep.meta.duplicate = true;
        }
        /** dep from assetToScopeHoist has the duplicate if it's not an inline require */
      } else if (!dep.meta.shouldWrap) {
        depToScopeHoist.meta.duplicate = true;
      }

      const previousIncomingDependencies = nullthrows(
        incomingDependencyMap.get(asset),
      );
      const incomingDependencies = previousIncomingDependencies.filter(
        (incomingDep) => incomingDep !== depToScopeHoist,
      );
      incomingDependencyMap.set(asset, incomingDependencies);
      queueAssetIfOneIncomingDependency(asset, incomingDependencies);
    }
  }
}
