"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.addJSMonolithBundle = addJSMonolithBundle;
function _featureFlags() {
  const data = require("@atlaspack/feature-flags");
  _featureFlags = function () {
    return data;
  };
  return data;
}
function _nullthrows() {
  const data = _interopRequireDefault(require("nullthrows"));
  _nullthrows = function () {
    return data;
  };
  return data;
}
function _interopRequireDefault(e) { return e && e.__esModule ? e : { default: e }; }
function addJSMonolithBundle(bundleGraph, entryAsset, entryDep) {
  let target = (0, _nullthrows().default)(entryDep.target, 'Expected dependency to have a valid target');

  // Create a single bundle to hold all JS assets
  let bundle = bundleGraph.createBundle({
    entryAsset,
    target,
    needsStableName: (0, _featureFlags().getFeatureFlag)('singleFileOutputStableName')
  });
  bundleGraph.traverse((node, _, actions) => {
    // JS assets can be added to the bundle, but the rest are ignored
    if (node.type === 'asset' && node.value.type === 'js') {
      bundleGraph.addAssetToBundle(node.value, bundle);
      return;
    }
    if (node.type !== 'dependency') {
      return;
    }
    let dependency = node.value;
    if (dependency.priority === 'lazy') {
      // Any async dependencies need to be internalized into the bundle, and will
      // be included by the asset check above
      bundleGraph.internalizeAsyncDependency(bundle, dependency);
      return;
    }
    let assets = bundleGraph.getDependencyAssets(dependency);
    for (const asset of assets) {
      if (asset.bundleBehavior === 'isolated' || asset.bundleBehavior === 'inlineIsolated') {
        throw new Error(`${asset.bundleBehavior === 'isolated' ? 'Isolated' : 'Inline isolated'} assets are not supported for single file output builds`);
      }

      // For assets marked as inline, we create new bundles and let other
      // plugins like optimizers include them in the primary bundle
      if (asset.bundleBehavior === 'inline') {
        // Create a new bundle to hold the isolated asset
        let isolatedBundle = bundleGraph.createBundle({
          entryAsset: asset,
          target,
          bundleBehavior: asset.bundleBehavior
        });
        bundleGraph.addAssetToBundle(asset, isolatedBundle);

        // Add the new bundle to the bundle graph, in its own bundle group
        bundleGraph.createBundleReference(bundle, isolatedBundle);
        bundleGraph.addBundleToBundleGroup(isolatedBundle, bundleGraph.createBundleGroup(dependency, target));

        // Nothing below the isolated asset needs to go in the main bundle, so
        // we can stop traversal here
        actions.skipChildren();

        // To be properly isolated, all of this asset's dependencies need to go
        // in this new bundle
        bundleGraph.traverse(subNode => {
          if (subNode.type === 'asset' && subNode.value.type === 'js') {
            bundleGraph.addAssetToBundle(subNode.value, isolatedBundle);
            return;
          }
        }, asset, {
          skipUnusedDependencies: true
        });
      }
    }
  }, entryAsset, {
    skipUnusedDependencies: true
  });
  let bundleGroup = bundleGraph.createBundleGroup(entryDep, target);
  bundleGraph.addBundleToBundleGroup(bundle, bundleGroup);
}