// @flow strict-local

import {ALL_EDGE_TYPES, type NodeId} from '@atlaspack/graph';
import type {
  Bundle as LegacyBundle,
  BundleGroup,
  MutableBundleGraph,
} from '@atlaspack/types';
import {getFeatureFlag} from '@atlaspack/feature-flags';
import invariant from 'assert';
import nullthrows from 'nullthrows';

import type {Bundle, IdealGraph} from './idealGraph';
import {idealBundleGraphEdges} from './idealGraph';

export function decorateLegacyGraph(
  idealGraph: IdealGraph,
  bundleGraph: MutableBundleGraph,
): void {
  let idealBundleToLegacyBundle: Map<Bundle, LegacyBundle> = new Map();

  let {
    bundleGraph: idealBundleGraph,
    dependencyBundleGraph,
    bundleGroupBundleIds,
    manualAssetToBundle,
  } = idealGraph;
  let entryBundleToBundleGroup: Map<NodeId, BundleGroup> = new Map();
  // Step Create Bundles: Create bundle groups, bundles, and shared bundles and add assets to them
  for (let [bundleNodeId, idealBundle] of idealBundleGraph.nodes.entries()) {
    if (!idealBundle || idealBundle === 'root') continue;
    let entryAsset = idealBundle.mainEntryAsset;
    let bundleGroup;
    let bundle;

    if (bundleGroupBundleIds.has(bundleNodeId)) {
      invariant(
        idealBundle.manualSharedBundle == null,
        'Processing a manualSharedBundle as a BundleGroup',
      );
      let dependencies = dependencyBundleGraph
        .getNodeIdsConnectedTo(
          dependencyBundleGraph.getNodeIdByContentKey(String(bundleNodeId)),
          ALL_EDGE_TYPES,
        )
        .map((nodeId) => {
          let dependency = nullthrows(dependencyBundleGraph.getNode(nodeId));
          invariant(dependency.type === 'dependency');
          return dependency.value;
        });
      invariant(
        entryAsset != null,
        'Processing a bundleGroup with no entry asset',
      );
      for (let dependency of dependencies) {
        bundleGroup = bundleGraph.createBundleGroup(
          dependency,
          idealBundle.target,
        );
      }
      invariant(bundleGroup);
      entryBundleToBundleGroup.set(bundleNodeId, bundleGroup);

      bundle = nullthrows(
        bundleGraph.createBundle({
          entryAsset: nullthrows(entryAsset),
          needsStableName: idealBundle.needsStableName,
          bundleBehavior: idealBundle.bundleBehavior,
          target: idealBundle.target,
          manualSharedBundle: idealBundle.manualSharedBundle,
        }),
      );

      bundleGraph.addBundleToBundleGroup(bundle, bundleGroup);
    } else if (
      idealBundle.sourceBundles.size > 0 &&
      !idealBundle.mainEntryAsset
    ) {
      let uniqueKey =
        idealBundle.uniqueKey != null
          ? idealBundle.uniqueKey
          : [...idealBundle.assets].map((asset) => asset.id).join(',');

      bundle = nullthrows(
        bundleGraph.createBundle({
          uniqueKey,
          needsStableName: idealBundle.needsStableName,
          bundleBehavior: idealBundle.bundleBehavior,
          type: idealBundle.type,
          target: idealBundle.target,
          env: idealBundle.env,
          manualSharedBundle: idealBundle.manualSharedBundle,
        }),
      );
    } else if (idealBundle.uniqueKey != null) {
      bundle = nullthrows(
        bundleGraph.createBundle({
          uniqueKey: idealBundle.uniqueKey,
          needsStableName: idealBundle.needsStableName,
          bundleBehavior: idealBundle.bundleBehavior,
          type: idealBundle.type,
          target: idealBundle.target,
          env: idealBundle.env,
          manualSharedBundle: idealBundle.manualSharedBundle,
        }),
      );
    } else {
      invariant(entryAsset != null);
      bundle = nullthrows(
        bundleGraph.createBundle({
          entryAsset,
          needsStableName: idealBundle.needsStableName,
          bundleBehavior: idealBundle.bundleBehavior,
          target: idealBundle.target,
          manualSharedBundle: idealBundle.manualSharedBundle,
        }),
      );
    }

    idealBundleToLegacyBundle.set(idealBundle, bundle);

    for (let asset of idealBundle.assets) {
      bundleGraph.addAssetToBundle(asset, bundle);
    }
  }
  // Step Internalization: Internalize dependencies for bundles
  for (let idealBundle of idealBundleGraph.nodes) {
    if (!idealBundle || idealBundle === 'root') continue;
    let bundle = nullthrows(idealBundleToLegacyBundle.get(idealBundle));
    if (idealBundle.internalizedAssets) {
      idealBundle.internalizedAssets.forEach((internalized) => {
        let incomingDeps = bundleGraph.getIncomingDependencies(
          idealGraph.assets[internalized],
        );
        for (let incomingDep of incomingDeps) {
          if (
            incomingDep.priority === 'lazy' &&
            incomingDep.specifierType !== 'url' &&
            bundle.hasDependency(incomingDep)
          ) {
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
      if (
        incomingDep.priority === 'lazy' &&
        incomingDep.specifierType !== 'url'
      ) {
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
    let entryBundle = nullthrows(idealBundleGraph.getNode(nodeId));
    invariant(entryBundle !== 'root');
    let legacyEntryBundle = nullthrows(
      idealBundleToLegacyBundle.get(entryBundle),
    );

    for (let id of outboundNodeIds) {
      let siblingBundle = nullthrows(idealBundleGraph.getNode(id));
      invariant(siblingBundle !== 'root');
      let legacySiblingBundle = nullthrows(
        idealBundleToLegacyBundle.get(siblingBundle),
      );
      bundleGraph.createBundleReference(legacyEntryBundle, legacySiblingBundle);
    }
  });

  // Step References: Add references to all bundles
  for (let [asset, references] of idealGraph.assetReference) {
    for (let [dependency, bundle] of references) {
      let legacyBundle = nullthrows(idealBundleToLegacyBundle.get(bundle));
      bundleGraph.createAssetReference(dependency, asset, legacyBundle);
    }
  }

  for (let {type, from, to} of idealBundleGraph.getAllEdges()) {
    let sourceBundle = nullthrows(idealBundleGraph.getNode(from));
    if (sourceBundle === 'root') {
      continue;
    }
    invariant(sourceBundle !== 'root');

    let legacySourceBundle = nullthrows(
      idealBundleToLegacyBundle.get(sourceBundle),
    );

    let targetBundle = nullthrows(idealBundleGraph.getNode(to));
    if (targetBundle === 'root') {
      continue;
    }
    invariant(targetBundle !== 'root');
    let legacyTargetBundle = nullthrows(
      idealBundleToLegacyBundle.get(targetBundle),
    );
    if (
      getFeatureFlag('conditionalBundlingApi') &&
      type === idealBundleGraphEdges.conditional
    ) {
      bundleGraph.createBundleConditionalReference(
        legacySourceBundle,
        legacyTargetBundle,
      );
    } else {
      bundleGraph.createBundleReference(legacySourceBundle, legacyTargetBundle);
    }
  }
}
