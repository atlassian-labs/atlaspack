"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.decorateLegacyGraph = decorateLegacyGraph;
function _graph() {
  const data = require("@atlaspack/graph");
  _graph = function () {
    return data;
  };
  return data;
}
function _featureFlags() {
  const data = require("@atlaspack/feature-flags");
  _featureFlags = function () {
    return data;
  };
  return data;
}
function _assert() {
  const data = _interopRequireDefault(require("assert"));
  _assert = function () {
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
var _idealGraph = require("./idealGraph");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { default: e }; }
function decorateLegacyGraph(idealGraph, bundleGraph) {
  let idealBundleToLegacyBundle = new Map();
  let {
    bundleGraph: idealBundleGraph,
    dependencyBundleGraph,
    bundleGroupBundleIds,
    manualAssetToBundle
  } = idealGraph;
  // This line can be deleted once supportWebpackChunkName feature flag is removed.
  let entryBundleToBundleGroup = new Map();
  // Step Create Bundles: Create bundle groups, bundles, and shared bundles and add assets to them
  for (let [bundleNodeId, idealBundle] of idealBundleGraph.nodes.entries()) {
    if (!idealBundle || idealBundle === 'root') continue;
    let entryAsset = idealBundle.mainEntryAsset;
    // This line can be deleted once supportWebpackChunkName feature flag is removed.
    let bundleGroup;
    let bundle;
    if (bundleGroupBundleIds.has(bundleNodeId)) {
      (0, _assert().default)(idealBundle.manualSharedBundle == null, 'Processing a manualSharedBundle as a BundleGroup');
      let dependencies = dependencyBundleGraph.getNodeIdsConnectedTo(dependencyBundleGraph.getNodeIdByContentKey(String(bundleNodeId)), _graph().ALL_EDGE_TYPES).map(nodeId => {
        let dependency = (0, _nullthrows().default)(dependencyBundleGraph.getNode(nodeId));
        (0, _assert().default)(dependency.type === 'dependency');
        return dependency.value;
      });
      (0, _assert().default)(entryAsset != null, 'Processing a bundleGroup with no entry asset');
      let bundleGroups = new Map();
      for (let dependency of dependencies) {
        bundleGroup = bundleGraph.createBundleGroup(dependency, idealBundle.target);
        bundleGroups.set(bundleGroup.entryAssetId, bundleGroup);
      }
      if ((0, _featureFlags().getFeatureFlag)('supportWebpackChunkName')) {
        (0, _assert().default)(bundleGroups.size > 0, 'No bundle groups created');
      } else {
        (0, _assert().default)(bundleGroup);
        entryBundleToBundleGroup.set(bundleNodeId, bundleGroup);
      }
      bundle = (0, _nullthrows().default)(bundleGraph.createBundle({
        entryAsset: (0, _nullthrows().default)(entryAsset),
        bundleRoots: Array.from(idealBundle.bundleRoots),
        needsStableName: idealBundle.needsStableName,
        bundleBehavior: idealBundle.bundleBehavior,
        target: idealBundle.target,
        manualSharedBundle: idealBundle.manualSharedBundle
      }));
      if ((0, _featureFlags().getFeatureFlag)('supportWebpackChunkName')) {
        for (let bundleGroup of bundleGroups.values()) {
          bundleGraph.addBundleToBundleGroup(bundle, bundleGroup);
        }
      } else {
        (0, _assert().default)(bundleGroup);
        bundleGraph.addBundleToBundleGroup(bundle, bundleGroup);
      }
    } else if (idealBundle.sourceBundles.size > 0 && !idealBundle.mainEntryAsset) {
      let uniqueKey = idealBundle.uniqueKey != null ? idealBundle.uniqueKey : [...idealBundle.assets].map(asset => asset.id).join(',');
      bundle = (0, _nullthrows().default)(bundleGraph.createBundle({
        uniqueKey,
        needsStableName: idealBundle.needsStableName,
        bundleBehavior: idealBundle.bundleBehavior,
        type: idealBundle.type,
        target: idealBundle.target,
        env: idealBundle.env,
        manualSharedBundle: idealBundle.manualSharedBundle
      }));
    } else if (idealBundle.uniqueKey != null) {
      bundle = (0, _nullthrows().default)(bundleGraph.createBundle({
        uniqueKey: idealBundle.uniqueKey,
        needsStableName: idealBundle.needsStableName,
        bundleBehavior: idealBundle.bundleBehavior,
        type: idealBundle.type,
        target: idealBundle.target,
        env: idealBundle.env,
        manualSharedBundle: idealBundle.manualSharedBundle
      }));
    } else {
      (0, _assert().default)(entryAsset != null);
      bundle = (0, _nullthrows().default)(bundleGraph.createBundle({
        entryAsset,
        bundleRoots: Array.from(idealBundle.bundleRoots),
        needsStableName: idealBundle.needsStableName,
        bundleBehavior: idealBundle.bundleBehavior,
        target: idealBundle.target,
        manualSharedBundle: idealBundle.manualSharedBundle
      }));
    }
    idealBundleToLegacyBundle.set(idealBundle, bundle);
    for (let asset of idealBundle.assets) {
      bundleGraph.addAssetToBundle(asset, bundle);
    }
  }
  // Step Internalization: Internalize dependencies for bundles
  for (let idealBundle of idealBundleGraph.nodes) {
    if (!idealBundle || idealBundle === 'root') continue;
    let bundle = (0, _nullthrows().default)(idealBundleToLegacyBundle.get(idealBundle));
    if (idealBundle.internalizedAssets) {
      idealBundle.internalizedAssets.forEach(internalized => {
        let incomingDeps = bundleGraph.getIncomingDependencies(idealGraph.assets[internalized]);
        for (let incomingDep of incomingDeps) {
          if (incomingDep.priority === 'lazy' && incomingDep.specifierType !== 'url' && bundle.hasDependency(incomingDep)) {
            bundleGraph.internalizeAsyncDependency(bundle, incomingDep);
          }
        }
      });
    }
  }
  // Manual Shared Bundles
  // NOTE: This only works under the assumption that manual shared bundles would have
  // always already been loaded before the bundle that requires internalization.
  for (let manualSharedAsset of manualAssetToBundle.keys()) {
    let incomingDeps = bundleGraph.getIncomingDependencies(manualSharedAsset);
    for (let incomingDep of incomingDeps) {
      if (incomingDep.priority === 'lazy' && incomingDep.specifierType !== 'url') {
        let bundles = bundleGraph.getBundlesWithDependency(incomingDep);
        for (let bundle of bundles) {
          bundleGraph.internalizeAsyncDependency(bundle, incomingDep);
        }
      }
    }
  }

  // Step Add to BundleGroups: Add bundles to their bundle groups
  idealBundleGraph.traverse((nodeId, _, actions) => {
    let node = idealBundleGraph.getNode(nodeId);
    if (node === 'root') {
      return;
    }
    actions.skipChildren();
    let outboundNodeIds = idealBundleGraph.getNodeIdsConnectedFrom(nodeId);
    let entryBundle = (0, _nullthrows().default)(idealBundleGraph.getNode(nodeId));
    (0, _assert().default)(entryBundle !== 'root');
    let legacyEntryBundle = (0, _nullthrows().default)(idealBundleToLegacyBundle.get(entryBundle));
    for (let id of outboundNodeIds) {
      let siblingBundle = (0, _nullthrows().default)(idealBundleGraph.getNode(id));
      (0, _assert().default)(siblingBundle !== 'root');
      let legacySiblingBundle = (0, _nullthrows().default)(idealBundleToLegacyBundle.get(siblingBundle));
      bundleGraph.createBundleReference(legacyEntryBundle, legacySiblingBundle);
    }
  });

  // Step References: Add references to all bundles
  for (let [asset, references] of idealGraph.assetReference) {
    for (let [dependency, bundle] of references) {
      let legacyBundle = (0, _nullthrows().default)(idealBundleToLegacyBundle.get(bundle));
      bundleGraph.createAssetReference(dependency, asset, legacyBundle);
    }
  }

  // @ts-expect-error TS2488
  for (let {
    type,
    from,
    to
  } of idealBundleGraph.getAllEdges()) {
    let sourceBundle = (0, _nullthrows().default)(idealBundleGraph.getNode(from));
    if (sourceBundle === 'root') {
      continue;
    }
    // @ts-expect-error TS2367
    (0, _assert().default)(sourceBundle !== 'root');
    let legacySourceBundle = (0, _nullthrows().default)(idealBundleToLegacyBundle.get(sourceBundle));
    let targetBundle = (0, _nullthrows().default)(idealBundleGraph.getNode(to));
    if (targetBundle === 'root') {
      continue;
    }
    // @ts-expect-error TS2367
    (0, _assert().default)(targetBundle !== 'root');
    let legacyTargetBundle = (0, _nullthrows().default)(idealBundleToLegacyBundle.get(targetBundle));
    if ((0, _featureFlags().getFeatureFlag)('conditionalBundlingApi') && type === _idealGraph.idealBundleGraphEdges.conditional) {
      bundleGraph.createBundleConditionalReference(legacySourceBundle, legacyTargetBundle);
    } else {
      bundleGraph.createBundleReference(legacySourceBundle, legacyTargetBundle);
    }
  }
}