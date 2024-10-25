// @flow strict-local
import type {Asset, Dependency, MutableBundleGraph} from '@atlaspack/types';
import nullthrows from 'nullthrows';

export function addJSMonolithBundle(
  bundleGraph: MutableBundleGraph,
  entryAsset: Asset,
  entryDep: Dependency,
): void {
  let target = nullthrows(
    entryDep.target,
    'Expected dependency to have a valid target',
  );

  // Create a single bundle to hold all JS assets
  let bundle = bundleGraph.createBundle({entryAsset, target});

  bundleGraph.traverse((node) => {
    // JS assets can be added to the bundle, but the rest are ignored
    if (node.type === 'asset' && node.value.type === 'js') {
      bundleGraph.addAssetToBundle(node.value, bundle);
    } else if (
      node.type === 'dependency' &&
      node.value.priority === 'lazy' &&
      !bundleGraph.isDependencySkipped(node.value)
    ) {
      // Any async dependencies need to be internalized into the bundle, and will
      // be included by the asset check above
      bundleGraph.internalizeAsyncDependency(bundle, node.value);
    }
  }, entryAsset);

  let bundleGroup = bundleGraph.createBundleGroup(entryDep, target);

  bundleGraph.addBundleToBundleGroup(bundle, bundleGroup);
}
