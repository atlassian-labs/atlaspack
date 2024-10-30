// @flow strict-local

import {
  ALL_EDGE_TYPES,
  type BitSet,
  type Graph,
  type NodeId,
} from '@atlaspack/graph';
import type {
  Bundle as LegacyBundle,
  MutableBundleGraph,
} from '@atlaspack/types';
import type {DefaultMap} from '@atlaspack/utils';
import invariant from 'assert';
import nullthrows from 'nullthrows';

export type Bundle = {|
  uniqueKey: ?string,
  assets: Set<Asset>,
  internalizedAssets?: BitSet,
  bundleBehavior?: ?BundleBehavior,
  needsStableName: boolean,
  mainEntryAsset: ?Asset,
  size: number,
  sourceBundles: Set<NodeId>,
  target: Target,
  env: Environment,
  type: string,
  manualSharedBundle: ?string, // for naming purposes
|};

export type DependencyBundleGraph = ContentGraph<
  | {|
      value: Bundle,
      type: 'bundle',
    |}
  | {|
      value: Dependency,
      type: 'dependency',
    |},
  number,
>;

// IdealGraph is the structure we will pass to decorate,
// which mutates the assetGraph into the bundleGraph we would
// expect from default bundler
export type IdealGraph = {|
  assets: Array<Asset>,
  dependencyBundleGraph: DependencyBundleGraph,
  bundleGraph: Graph<Bundle | 'root'>,
  bundleGroupBundleIds: Set<NodeId>,
  assetReference: DefaultMap<Asset, Array<[Dependency, Bundle]>>,
  manualAssetToBundle: Map<Asset, NodeId>,
|};

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

      // If a manual bundle is not referenced anywhere (per the idealBundle.sourceBundles.size > 0
      // check above), then we must add it to the graph ourselves like a bundle group.
      if (idealBundle.manualSharedBundle) {
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

        for (let dependency of dependencies) {
          bundleGroup = bundleGraph.createBundleGroup(
            dependency,
            idealBundle.target,
          );
        }

        invariant(bundleGroup);
        bundleGraph.addBundleToBundleGroup(bundle, bundleGroup);
      }
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

  for (let {from, to} of idealBundleGraph.getAllEdges()) {
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

    let legacyTargetBundle = nullthrows(
      idealBundleToLegacyBundle.get(targetBundle),
    );

    bundleGraph.createBundleReference(legacySourceBundle, legacyTargetBundle);
  }
}
