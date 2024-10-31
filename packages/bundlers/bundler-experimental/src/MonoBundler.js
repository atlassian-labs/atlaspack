// @flow strict-local

import type {
  BundleGraph,
  Bundle,
  MutableBundleGraph,
  Asset,
  Dependency,
} from '@atlaspack/types';

export type MonoBundlerInput<B: Bundle> = {|
  inputGraph: BundleGraph<B>,
  outputGraph: MutableBundleGraph,
  entries: {|entryAsset: Asset, entryDependency: Dependency|}[],
|};

export type MonoBundlerOutput = {|
  bundleGraph: BundleGraph<Bundle>,
|};

/**
 * A bundler that puts all assets into a single output bundle.
 */
export function monoBundler<B: Bundle>({
  inputGraph,
  outputGraph,
  entries,
}: MonoBundlerInput<B>): MonoBundlerOutput {
  for (let {entryAsset, entryDependency} of entries) {
    const target = entryDependency.target;
    if (!target) {
      throw new Error('Expected dependency to have a valid target');
    }

    const bundle = outputGraph.createBundle({
      entryAsset,
      target,
    });

    inputGraph.traverse(
      (node) => {
        if (node.type === 'asset' && node.value.type === 'js') {
          outputGraph.addAssetToBundle(node.value, bundle);
          return;
        }

        if (node.type === 'dependency' && node.value.priority === 'lazy') {
          // Any async dependencies need to be internalized into the bundle, and will
          // be included by the asset check above
          outputGraph.internalizeAsyncDependency(bundle, node.value);
        }
      },
      entryAsset,
      {
        skipUnusedDependencies: true,
      },
    );

    const bundleGroup = outputGraph.createBundleGroup(entryDependency, target);
    outputGraph.addBundleToBundleGroup(bundle, bundleGroup);
  }

  return {bundleGraph: outputGraph};
}
