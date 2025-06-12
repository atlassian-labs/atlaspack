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

  function queueAssetIfOneIncomingDependency(
    asset: Asset,
    incomingDependencies: Dependency[],
  ) {
    if (incomingDependencies.length === 1) {
      const [dependency] = incomingDependencies;
      if (dependency.priority !== 'lazy') {
        const assetToScopeHoistInto =
          assetGraph.getAssetWithDependency(dependency);
        // TODO: When is this undefined?
        if (assetToScopeHoistInto) {
          const isCircularDependency = assetGraph
            .getIncomingDependencies(assetToScopeHoistInto)
            .some((dep) => dep.sourceAssetId === asset.id);
          if (!isCircularDependency) {
            scopeHoistQueue.push({
              from: asset,
              to: assetToScopeHoistInto,
            });
          }
        }
      }
    }
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
    from.meta.inline = true;

    const assetToScopeHoistDependencies = new Set(
      assetGraph.getDependencies(from),
    );
    const assetToScopeHoistIntoDependencyAssets = new Set(
      assetGraph.getDependencies(to).map((dependency) => {
        return assetGraph.getResolvedAsset(dependency);
      }),
    );

    /**
     * Dependencies which are shared by `from` and `to` are removed from
     * the incoming dependency counts for their resolved assets.
     */
    for (const dependency of assetToScopeHoistDependencies) {
      const asset = assetGraph.getResolvedAsset(dependency);
      if (asset && assetToScopeHoistIntoDependencyAssets.has(asset)) {
        const previousIncomingDependencies = incomingDependencyMap.get(asset);
        if (previousIncomingDependencies !== undefined) {
          const incomingDependencies = previousIncomingDependencies.filter(
            (dep) => {
              return !assetToScopeHoistDependencies.has(dep);
            },
          );
          incomingDependencyMap.set(asset, incomingDependencies);
          queueAssetIfOneIncomingDependency(asset, incomingDependencies);
        }
      }
    }
  }
}
