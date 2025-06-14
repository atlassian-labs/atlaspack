// @flow strict-local
import type {MutableBundleGraph, Asset, Dependency} from '@atlaspack/types';

import nullthrows from 'nullthrows';
import path from 'path';

interface ScopeHoist {
  from: Asset;
  to: Asset;
}

/**
 * Add `meta.inline = true` for all assets which have only one incomingDependency until
 * no more assets have one incomingDependency
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
          if (!isCircularDependency && !hasReExport(dependency)) {
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
    /**
     * This metadata is used by ScopehoistingPackager. Removing this
     * line will disable inlining within wrapped assets. All tests
     * will pass except for the inline wrapped test with this disabled.
     */
    from.meta.inline = to.id;

    const assetToScopeHoistAssetDependencies =
      getSyncResolvedAssetDependencies(from);
    const assetToScopeHoistIntoAssetDependencies =
      getSyncResolvedAssetDependencies(to);

    /**
     * All assets which are used in both `from` and `to` should have their
     * incomingDependencies updated.
     */
    for (const [
      assetToScopeHoist,
      depToScopeHoist,
    ] of assetToScopeHoistAssetDependencies) {
      const duplicateDep =
        assetToScopeHoistIntoAssetDependencies.get(assetToScopeHoist);
      if (duplicateDep !== undefined) {
        duplicateDep.meta.duplicate = true;
        depToScopeHoist.meta.real = true;
        console.log(
          duplicateDep.specifier,
          'in',
          path.basename(
            assetGraph.getAssetById(duplicateDep.sourceAssetId).filePath,
          ),
          'is a duplicate of',
          depToScopeHoist.specifier,
          'in',
          path.basename(
            assetGraph.getAssetById(depToScopeHoist.sourceAssetId).filePath,
          ),
        );

        const previousIncomingDependencies =
          incomingDependencyMap.get(assetToScopeHoist);
        const incomingDependencies = previousIncomingDependencies.filter(
          (incomingDep) => incomingDep !== depToScopeHoist,
        );
        incomingDependencyMap.set(assetToScopeHoist, incomingDependencies);
        queueAssetIfOneIncomingDependency(
          assetToScopeHoist,
          incomingDependencies,
        );
      }
    }
  }
}
