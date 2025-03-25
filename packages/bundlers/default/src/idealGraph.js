// @flow strict-local

import path from 'path';

import {getFeatureFlag} from '@atlaspack/feature-flags';
import {
  ALL_EDGE_TYPES,
  BitSet,
  ContentGraph,
  Graph,
  type NodeId,
} from '@atlaspack/graph';
import type {
  Asset,
  BundleBehavior,
  Dependency,
  Environment,
  MutableBundleGraph,
  Target,
  PluginLogger,
} from '@atlaspack/types';
import {DefaultMap, globToRegex} from '@atlaspack/utils';
import invariant from 'assert';
import nullthrows from 'nullthrows';

import type {ResolvedBundlerConfig} from './bundlerConfig';

/* BundleRoot - An asset that is the main entry of a Bundle. */
type BundleRoot = Asset;

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

const dependencyPriorityEdges = {
  sync: 1,
  parallel: 2,
  lazy: 3,
  conditional: 4,
};

export const idealBundleGraphEdges = Object.freeze({
  default: 1,
  conditional: 2,
});

type IdealBundleGraph = Graph<
  Bundle | 'root',
  $Values<typeof idealBundleGraphEdges>,
>;

// IdealGraph is the structure we will pass to decorate,
// which mutates the assetGraph into the bundleGraph we would
// expect from default bundler
export type IdealGraph = {|
  assetReference: DefaultMap<Asset, Array<[Dependency, Bundle]>>,
  assets: Array<Asset>,
  bundleGraph: IdealBundleGraph,
  bundleGroupBundleIds: Set<NodeId>,
  dependencyBundleGraph: DependencyBundleGraph,
  manualAssetToBundle: Map<Asset, NodeId>,
|};

export function createIdealGraph(
  assetGraph: MutableBundleGraph,
  config: ResolvedBundlerConfig,
  entries: Map<Asset, Dependency>,
  logger: PluginLogger,
): IdealGraph {
  // Asset to the bundle and group it's an entry of
  let bundleRoots: Map<BundleRoot, [NodeId, NodeId]> = new Map();
  let bundles: Map<string, NodeId> = new Map();
  let dependencyBundleGraph: DependencyBundleGraph = new ContentGraph();
  let assetReference: DefaultMap<
    Asset,
    Array<[Dependency, Bundle]>,
  > = new DefaultMap(() => []);

  // A Graph of Bundles and a root node (dummy string), which models only Bundles, and connections to their
  // referencing Bundle. There are no actual BundleGroup nodes, just bundles that take on that role.
  let bundleGraph: IdealBundleGraph = new Graph();
  let stack: Array<[BundleRoot, NodeId]> = [];

  let bundleRootEdgeTypes = {
    parallel: 1,
    lazy: 2,
  };
  // Graph that models bundleRoots, with parallel & async deps only to inform reachability
  let bundleRootGraph: Graph<
    number, // asset index
    $Values<typeof bundleRootEdgeTypes>,
  > = new Graph();
  let assetToBundleRootNodeId = new Map<BundleRoot, number>();

  let bundleGroupBundleIds: Set<NodeId> = new Set();

  let bundleGraphRootNodeId = nullthrows(bundleGraph.addNode('root'));
  bundleGraph.setRootNodeId(bundleGraphRootNodeId);
  // Step Create Entry Bundles
  for (let [asset, dependency] of entries) {
    let bundle = createBundle({
      asset,
      target: nullthrows(dependency.target),
      needsStableName: dependency.isEntry,
    });
    let nodeId = bundleGraph.addNode(bundle);
    bundles.set(asset.id, nodeId);
    bundleRoots.set(asset, [nodeId, nodeId]);
    bundleGraph.addEdge(bundleGraphRootNodeId, nodeId);

    dependencyBundleGraph.addEdge(
      dependencyBundleGraph.addNodeByContentKeyIfNeeded(dependency.id, {
        value: dependency,
        type: 'dependency',
      }),
      dependencyBundleGraph.addNodeByContentKeyIfNeeded(String(nodeId), {
        value: bundle,
        type: 'bundle',
      }),
      dependencyPriorityEdges[dependency.priority],
    );
    bundleGroupBundleIds.add(nodeId);
  }

  let assets = [];
  let assetToIndex = new Map<Asset, number>();

  function makeManualAssetToConfigLookup() {
    let manualAssetToConfig = new Map();
    let constantModuleToMSB = new DefaultMap(() => []);

    if (config.manualSharedBundles.length === 0) {
      return {manualAssetToConfig, constantModuleToMSB};
    }

    let parentsToConfig = new DefaultMap(() => []);

    for (let c of config.manualSharedBundles) {
      if (c.root != null) {
        parentsToConfig.get(path.join(config.projectRoot, c.root)).push(c);
      }
    }
    let numParentsToFind = parentsToConfig.size;
    let configToParentAsset = new Map();

    assetGraph.traverse((node, _, actions) => {
      if (node.type === 'asset' && parentsToConfig.has(node.value.filePath)) {
        for (let c of parentsToConfig.get(node.value.filePath)) {
          configToParentAsset.set(c, node.value);
        }

        numParentsToFind--;

        if (numParentsToFind === 0) {
          // If we've found all parents we can stop traversal
          actions.stop();
        }
      }
    });

    // Process in reverse order so earlier configs take precedence
    for (let c of config.manualSharedBundles.reverse()) {
      if (c.root != null && !configToParentAsset.has(c)) {
        logger.warn({
          origin: '@atlaspack/bundler-default',
          message: `Manual shared bundle "${c.name}" skipped, no root asset found`,
        });
        continue;
      }

      let parentAsset = configToParentAsset.get(c);
      let assetRegexes = c.assets.map((glob) => globToRegex(glob));

      assetGraph.traverse((node, _, actions) => {
        if (
          node.type === 'asset' &&
          (!Array.isArray(c.types) || c.types.includes(node.value.type))
        ) {
          let projectRelativePath = path.relative(
            config.projectRoot,
            node.value.filePath,
          );
          if (!assetRegexes.some((regex) => regex.test(projectRelativePath))) {
            return;
          }

          // We track all matching MSB's for constant modules as they are never duplicated
          // and need to be assigned to all matching bundles
          if (node.value.meta.isConstantModule === true) {
            constantModuleToMSB.get(node.value).push(c);
          }
          manualAssetToConfig.set(node.value, c);
          return;
        }

        if (
          node.type === 'dependency' &&
          (node.value.priority === 'lazy' ||
            (getFeatureFlag('conditionalBundlingApi') &&
              node.value.priority === 'conditional')) &&
          parentAsset
        ) {
          // Don't walk past the bundle group assets
          actions.skipChildren();
        }
      }, parentAsset);
    }

    return {manualAssetToConfig, constantModuleToMSB};
  }

  //Manual is a map of the user-given name to the bundle node Id that corresponds to ALL the assets that match any glob in that user-specified array
  let manualSharedMap: Map<string, NodeId> = new Map();
  // May need a map to be able to look up NON- bundle root assets which need special case instructions
  // Use this when placing assets into bundles, to avoid duplication
  let manualAssetToBundle: Map<Asset, NodeId> = new Map();
  let {manualAssetToConfig, constantModuleToMSB} =
    makeManualAssetToConfigLookup();
  let manualBundleToInternalizedAsset: DefaultMap<
    NodeId,
    Array<Asset>,
  > = new DefaultMap(() => []);

  /**
   * Step Create Bundles: Traverse the assetGraph (aka MutableBundleGraph) and create bundles
   * for asset type changes, parallel, inline, and async or lazy dependencies,
   * adding only that asset to each bundle, not its entire subgraph.
   */
  assetGraph.traverse(
    {
      enter(node, context, actions) {
        if (node.type === 'asset') {
          if (
            context?.type === 'dependency' &&
            context?.value.isEntry &&
            !entries.has(node.value)
          ) {
            // Skip whole subtrees of other targets by skipping those entries
            actions.skipChildren();
            return node;
          }
          assetToIndex.set(node.value, assets.length);
          assets.push(node.value);

          let bundleIdTuple = bundleRoots.get(node.value);
          if (bundleIdTuple && bundleIdTuple[0] === bundleIdTuple[1]) {
            // Push to the stack (only) when a new bundle is created
            stack.push([node.value, bundleIdTuple[0]]);
          } else if (bundleIdTuple) {
            // Otherwise, push on the last bundle that marks the start of a BundleGroup
            stack.push([node.value, stack[stack.length - 1][1]]);
          }
        } else if (node.type === 'dependency') {
          if (context == null) {
            return node;
          }
          let dependency = node.value;

          invariant(context?.type === 'asset');

          let assets = assetGraph.getDependencyAssets(dependency);
          if (assets.length === 0) {
            return node;
          }

          for (let childAsset of assets) {
            let bundleId = bundles.get(childAsset.id);
            let bundle;

            // MSB Step 1: Match glob on filepath and type for any asset
            let manualSharedBundleKey;
            let manualSharedObject = manualAssetToConfig.get(childAsset);

            if (manualSharedObject) {
              // MSB Step 2: Generate a key for which to look up this manual bundle with
              manualSharedBundleKey =
                manualSharedObject.name + ',' + childAsset.type;
            }

            if (
              // MSB Step 3: If a bundle for these globs already exists, use it
              manualSharedBundleKey != null &&
              manualSharedMap.has(manualSharedBundleKey)
            ) {
              bundleId = nullthrows(manualSharedMap.get(manualSharedBundleKey));
            }
            if (
              dependency.priority === 'lazy' ||
              (getFeatureFlag('conditionalBundlingApi') &&
                node.value.priority === 'conditional') ||
              childAsset.bundleBehavior === 'isolated' // An isolated Dependency, or Bundle must contain all assets it needs to load.
            ) {
              if (bundleId == null) {
                let firstBundleGroup = nullthrows(
                  bundleGraph.getNode(stack[0][1]),
                );
                invariant(firstBundleGroup !== 'root');
                bundle = createBundle({
                  asset: childAsset,
                  bundleBehavior:
                    dependency.bundleBehavior ?? childAsset.bundleBehavior,
                  needsStableName:
                    dependency.bundleBehavior === 'inline' ||
                    childAsset.bundleBehavior === 'inline'
                      ? false
                      : dependency.isEntry || dependency.needsStableName,
                  target: firstBundleGroup.target,
                });
                bundleId = bundleGraph.addNode(bundle);
                bundles.set(childAsset.id, bundleId);
                bundleRoots.set(childAsset, [bundleId, bundleId]);
                bundleGroupBundleIds.add(bundleId);
                bundleGraph.addEdge(bundleGraphRootNodeId, bundleId);
                if (manualSharedObject) {
                  // MSB Step 4: If this was the first instance of a match, mark mainAsset for internalization
                  // since MSBs should not have main entry assets
                  manualBundleToInternalizedAsset
                    .get(bundleId)
                    .push(childAsset);
                }
              } else {
                bundle = nullthrows(bundleGraph.getNode(bundleId));
                invariant(bundle !== 'root');

                if (
                  // If this dependency requests isolated, but the bundle is not,
                  // make the bundle isolated for all uses.
                  dependency.bundleBehavior === 'isolated' &&
                  bundle.bundleBehavior == null
                ) {
                  bundle.bundleBehavior = dependency.bundleBehavior;
                }
              }

              dependencyBundleGraph.addEdge(
                dependencyBundleGraph.addNodeByContentKeyIfNeeded(
                  dependency.id,
                  {
                    value: dependency,
                    type: 'dependency',
                  },
                ),
                dependencyBundleGraph.addNodeByContentKeyIfNeeded(
                  String(bundleId),
                  {
                    value: bundle,
                    type: 'bundle',
                  },
                ),
                dependencyPriorityEdges[dependency.priority],
              );

              if (
                getFeatureFlag('conditionalBundlingApi') &&
                dependency.priority === 'conditional'
              ) {
                let [referencingBundleRoot, bundleGroupNodeId] = nullthrows(
                  stack[stack.length - 1],
                );

                let referencingBundleId = nullthrows(
                  bundleRoots.get(referencingBundleRoot),
                )[0];

                if (
                  config.loadConditionalBundlesInParallel ??
                  !bundle.env.shouldScopeHoist
                ) {
                  // When configured (or serving code in development), serve conditional bundles in parallel so we don't get module not found errors
                  bundleRoots.set(childAsset, [bundleId, bundleGroupNodeId]);
                  bundleGraph.addEdge(referencingBundleId, bundleId);
                }

                // Add conditional edge to track which bundles request each other
                bundleGraph.addEdge(
                  referencingBundleId,
                  bundleId,
                  idealBundleGraphEdges.conditional,
                );
              }
            } else if (
              dependency.priority === 'parallel' ||
              childAsset.bundleBehavior === 'inline'
            ) {
              // The referencing bundleRoot is the root of a Bundle that first brings in another bundle (essentially the FIRST parent of a bundle, this may or may not be a bundleGroup)
              let [referencingBundleRoot, bundleGroupNodeId] = nullthrows(
                stack[stack.length - 1],
              );
              let bundleGroup = nullthrows(
                bundleGraph.getNode(bundleGroupNodeId),
              );
              invariant(bundleGroup !== 'root');

              let referencingBundleId = nullthrows(
                bundleRoots.get(referencingBundleRoot),
              )[0];
              let referencingBundle = nullthrows(
                bundleGraph.getNode(referencingBundleId),
              );
              invariant(referencingBundle !== 'root');

              if (bundleId == null) {
                bundle = createBundle({
                  // Bundles created from type changes shouldn't have an entry asset.
                  asset: childAsset,
                  type: childAsset.type,
                  env: childAsset.env,
                  bundleBehavior:
                    dependency.bundleBehavior ?? childAsset.bundleBehavior,
                  target: referencingBundle.target,
                  needsStableName:
                    childAsset.bundleBehavior === 'inline' ||
                    dependency.bundleBehavior === 'inline' ||
                    (dependency.priority === 'parallel' &&
                      !dependency.needsStableName)
                      ? false
                      : referencingBundle.needsStableName,
                });
                bundleId = bundleGraph.addNode(bundle);
              } else {
                bundle = bundleGraph.getNode(bundleId);
                invariant(bundle != null && bundle !== 'root');

                if (
                  // If this dependency requests isolated, but the bundle is not,
                  // make the bundle isolated for all uses.
                  dependency.bundleBehavior === 'isolated' &&
                  bundle.bundleBehavior == null
                ) {
                  bundle.bundleBehavior = dependency.bundleBehavior;
                }
              }

              bundles.set(childAsset.id, bundleId);

              // A bundle can belong to multiple bundlegroups, all the bundle groups of it's
              // ancestors, and all async and entry bundles before it are "bundle groups"
              // TODO: We may need to track bundles to all bundleGroups it belongs to in the future.
              bundleRoots.set(childAsset, [bundleId, bundleGroupNodeId]);
              bundleGraph.addEdge(referencingBundleId, bundleId);

              if (bundleId != bundleGroupNodeId) {
                dependencyBundleGraph.addEdge(
                  dependencyBundleGraph.addNodeByContentKeyIfNeeded(
                    dependency.id,
                    {
                      value: dependency,
                      type: 'dependency',
                    },
                  ),
                  dependencyBundleGraph.addNodeByContentKeyIfNeeded(
                    String(bundleId),
                    {
                      value: bundle,
                      type: 'bundle',
                    },
                  ),
                  dependencyPriorityEdges.parallel,
                );
              }

              assetReference.get(childAsset).push([dependency, bundle]);
            } else {
              bundleId = null;
            }
            if (manualSharedObject && bundleId != null) {
              // MSB Step 5:  At this point we've either created or found an existing MSB bundle
              // add the asset if it doesn't already have it and set key

              invariant(
                bundle !== 'root' && bundle != null && bundleId != null,
              );

              manualAssetToBundle.set(childAsset, bundleId);

              if (!bundle.assets.has(childAsset)) {
                // Add asset to bundle
                bundle.assets.add(childAsset);
                bundle.size += childAsset.stats.size;
              }

              bundles.set(childAsset.id, bundleId);
              bundleRoots.set(childAsset, [bundleId, bundleId]);

              invariant(manualSharedBundleKey != null);
              // Ensure we set key to BundleId so the next glob match uses the appropriate bundle
              if (!manualSharedMap.has(manualSharedBundleKey)) {
                manualSharedMap.set(manualSharedBundleKey, bundleId);
              }
              bundle.manualSharedBundle = manualSharedObject.name;
              bundle.uniqueKey = manualSharedObject.name + childAsset.type;
            }
          }
        }
        return node;
      },
      exit(node) {
        if (stack[stack.length - 1]?.[0] === node.value) {
          stack.pop();
        }
      },
    },
    null,
    {skipUnusedDependencies: true},
  );

  // Strip MSBs of entries
  for (let [
    nodeId,
    internalizedAssets,
  ] of manualBundleToInternalizedAsset.entries()) {
    let bundle = bundleGraph.getNode(nodeId);
    invariant(bundle != null && bundle !== 'root');

    if (!bundle.internalizedAssets) {
      bundle.internalizedAssets = new BitSet(assets.length);
    }
    for (let asset of internalizedAssets) {
      bundle.internalizedAssets.add(nullthrows(assetToIndex.get(asset)));
    }
    bundle.mainEntryAsset = null;
    bundleGroupBundleIds.delete(nodeId); // manual bundles can now act as shared, non-bundle group, should they be non-bundleRoots as well?
  }

  /**
   *  Step Determine Reachability: Determine reachability for every asset from each bundleRoot.
   * This is later used to determine which bundles to place each asset in. We build up two
   * structures, one traversal each. ReachableRoots to store sync relationships,
   * and bundleRootGraph to store the minimal availability through `parallel` and `async` relationships.
   * The two graphs, are used to build up ancestorAssets, a structure which holds all availability by
   * all means for each asset.
   */
  let rootNodeId = bundleRootGraph.addNode(-1);
  bundleRootGraph.setRootNodeId(rootNodeId);

  for (let [root] of bundleRoots) {
    let nodeId = bundleRootGraph.addNode(nullthrows(assetToIndex.get(root)));
    assetToBundleRootNodeId.set(root, nodeId);
    if (entries.has(root)) {
      bundleRootGraph.addEdge(rootNodeId, nodeId);
    }
  }

  // reachableRoots is an array of bit sets for each asset. Each bit set
  // indicates which bundle roots are reachable from that asset synchronously.
  let reachableRoots = [];
  for (let i = 0; i < assets.length; i++) {
    reachableRoots.push(new BitSet(bundleRootGraph.nodes.length));
  }

  // reachableAssets is the inverse mapping of reachableRoots. For each bundle root,
  // it contains a bit set that indicates which assets are reachable from it.
  let reachableAssets = [];

  // ancestorAssets maps bundle roots to the set of all assets available to it at runtime,
  // including in earlier parallel bundles. These are intersected through all paths to
  // the bundle to ensure that the available assets are always present no matter in which
  // order the bundles are loaded.
  let ancestorAssets = [];

  let inlineConstantDeps = new DefaultMap(() => new Set());

  for (let [bundleRootId, assetId] of bundleRootGraph.nodes.entries()) {
    let reachable = new BitSet(assets.length);
    reachableAssets.push(reachable);
    ancestorAssets.push(null);

    if (bundleRootId == rootNodeId || assetId == null) continue;
    // Add sync relationships to ReachableRoots
    let root = assets[assetId];
    assetGraph.traverse(
      (node, _, actions) => {
        if (node.value === root) {
          return;
        }
        if (node.type === 'dependency') {
          let dependency = node.value;

          if (
            dependency.priority !== 'sync' &&
            dependencyBundleGraph.hasContentKey(dependency.id)
          ) {
            let assets = assetGraph.getDependencyAssets(dependency);
            if (assets.length === 0) {
              return;
            }
            invariant(assets.length === 1);
            let bundleRoot = assets[0];
            let bundle = nullthrows(
              bundleGraph.getNode(nullthrows(bundles.get(bundleRoot.id))),
            );
            if (
              bundle !== 'root' &&
              bundle.bundleBehavior == null &&
              !bundle.env.isIsolated() &&
              bundle.env.context === root.env.context
            ) {
              bundleRootGraph.addEdge(
                bundleRootId,
                nullthrows(assetToBundleRootNodeId.get(bundleRoot)),
                dependency.priority === 'parallel' ||
                  ((config.loadConditionalBundlesInParallel ??
                    !bundle.env.shouldScopeHoist) &&
                    dependency.priority === 'conditional')
                  ? bundleRootEdgeTypes.parallel
                  : bundleRootEdgeTypes.lazy,
              );
            }
          }

          if (dependency.priority !== 'sync') {
            actions.skipChildren();
          }
          return;
        }
        //asset node type
        let asset = node.value;
        if (asset.bundleBehavior != null) {
          actions.skipChildren();
          return;
        }
        let assetIndex = nullthrows(assetToIndex.get(node.value));
        reachable.add(assetIndex);
        reachableRoots[assetIndex].add(bundleRootId);

        if (asset.meta.isConstantModule === true) {
          let parents = assetGraph
            .getIncomingDependencies(asset)
            .map((dep) => nullthrows(assetGraph.getAssetWithDependency(dep)));

          for (let parent of parents) {
            inlineConstantDeps.get(parent).add(asset);
          }
        }

        return;
      },
      root,
      {skipUnusedDependencies: true},
    );
  }

  for (let entry of entries.keys()) {
    // Initialize an empty set of ancestors available to entries
    let entryId = nullthrows(assetToBundleRootNodeId.get(entry));
    ancestorAssets[entryId] = new BitSet(assets.length);
  }

  // Step Determine Availability
  // Visit nodes in a topological order, visiting parent nodes before child nodes.

  // This allows us to construct an understanding of which assets will already be
  // loaded and available when a bundle runs, by pushing available assets downwards and
  // computing the intersection of assets available through all possible paths to a bundle.
  // We call this structure ancestorAssets, a Map that tracks a bundleRoot,
  // to all assets available to it (meaning they will exist guaranteed when the bundleRoot is loaded)
  //  The topological sort ensures all parents are visited before the node we want to process.
  for (let nodeId of bundleRootGraph.topoSort(ALL_EDGE_TYPES)) {
    if (nodeId === rootNodeId) continue;
    const bundleRoot = assets[nullthrows(bundleRootGraph.getNode(nodeId))];
    let bundleGroupId = nullthrows(bundleRoots.get(bundleRoot))[1];

    // At a BundleRoot, we access it's available assets (via ancestorAssets),
    // and add to that all assets within the bundles in that BundleGroup.

    // This set is available to all bundles in a particular bundleGroup because
    // bundleGroups are just bundles loaded at the same time. However it is
    // not true that a bundle's available assets = all assets of all the bundleGroups
    // it belongs to. It's the intersection of those sets.
    let available;
    if (bundleRoot.bundleBehavior === 'isolated') {
      available = new BitSet(assets.length);
    } else {
      available = nullthrows(ancestorAssets[nodeId]).clone();
      for (let bundleIdInGroup of [
        bundleGroupId,
        ...bundleGraph.getNodeIdsConnectedFrom(bundleGroupId),
      ]) {
        let bundleInGroup = nullthrows(bundleGraph.getNode(bundleIdInGroup));
        invariant(bundleInGroup !== 'root');
        if (bundleInGroup.bundleBehavior != null) {
          continue;
        }

        for (let bundleRoot of bundleInGroup.assets) {
          // Assets directly connected to current bundleRoot
          available.add(nullthrows(assetToIndex.get(bundleRoot)));
          available.union(
            reachableAssets[
              nullthrows(assetToBundleRootNodeId.get(bundleRoot))
            ],
          );
        }
      }
    }

    //  Now that we have bundleGroup availability, we will propagate that down to all the children
    //  of this bundleGroup. For a child, we also must maintain parallel availability. If it has
    //  parallel siblings that come before it, those, too, are available to it. Add those parallel
    //  available assets to the set of available assets for this child as well.
    let children = bundleRootGraph.getNodeIdsConnectedFrom(
      nodeId,
      ALL_EDGE_TYPES,
    );
    let parallelAvailability = new BitSet(assets.length);

    for (let childId of children) {
      let assetId = nullthrows(bundleRootGraph.getNode(childId));
      let child = assets[assetId];
      let bundleBehavior = getBundleFromBundleRoot(child).bundleBehavior;
      if (bundleBehavior != null) {
        continue;
      }
      let isParallel = bundleRootGraph.hasEdge(
        nodeId,
        childId,
        bundleRootEdgeTypes.parallel,
      );

      // Most of the time, a child will have many parent bundleGroups,
      // so the next time we peek at a child from another parent, we will
      // intersect the availability built there with the previously computed
      // availability. this ensures no matter which bundleGroup loads a particular bundle,
      // it will only assume availability of assets it has under any circumstance
      const childAvailableAssets = ancestorAssets[childId];
      let currentChildAvailable = isParallel
        ? BitSet.union(parallelAvailability, available)
        : available;
      if (childAvailableAssets != null) {
        childAvailableAssets.intersect(currentChildAvailable);
      } else {
        ancestorAssets[childId] = currentChildAvailable.clone();
      }
      if (isParallel) {
        parallelAvailability.union(reachableAssets[childId]);
        parallelAvailability.add(assetId); //The next sibling should have older sibling available via parallel
      }
    }
  }
  // Step Internalize async bundles - internalize Async bundles if and only if,
  // the bundle is synchronously available elsewhere.
  // We can query sync assets available via reachableRoots. If the parent has
  // the bundleRoot by reachableRoots AND ancestorAssets, internalize it.
  for (let [id, bundleRootId] of bundleRootGraph.nodes.entries()) {
    if (bundleRootId == null || id === rootNodeId) continue;
    let bundleRoot = assets[bundleRootId];

    if (manualAssetToConfig.has(bundleRoot)) {
      // We internalize for MSBs later, we should never delete MSBs
      continue;
    }

    let parentRoots = bundleRootGraph.getNodeIdsConnectedTo(id, ALL_EDGE_TYPES);
    let canDelete =
      getBundleFromBundleRoot(bundleRoot).bundleBehavior !== 'isolated';
    if (parentRoots.length === 0) continue;
    for (let parentId of parentRoots) {
      if (parentId === rootNodeId) {
        // connected to root.
        canDelete = false;
        continue;
      }
      if (
        reachableAssets[parentId].has(bundleRootId) ||
        ancestorAssets[parentId]?.has(bundleRootId)
      ) {
        let parentAssetId = nullthrows(bundleRootGraph.getNode(parentId));
        let parent = assets[parentAssetId];
        let parentBundle = bundleGraph.getNode(
          nullthrows(bundles.get(parent.id)),
        );
        invariant(parentBundle != null && parentBundle !== 'root');
        if (!parentBundle.internalizedAssets) {
          parentBundle.internalizedAssets = new BitSet(assets.length);
        }

        parentBundle.internalizedAssets.add(bundleRootId);
      } else {
        canDelete = false;
      }
    }
    if (canDelete) {
      deleteBundle(bundleRoot);
    }
  }

  function assignInlineConstants(parentAsset: Asset, bundle: Bundle) {
    for (let inlineConstant of inlineConstantDeps.get(parentAsset)) {
      if (!bundle.assets.has(inlineConstant)) {
        bundle.assets.add(inlineConstant);
        bundle.size += inlineConstant.stats.size;
      }
    }
  }

  // Step Insert Or Share: Place all assets into bundles or create shared bundles. Each asset
  // is placed into a single bundle based on the bundle entries it is reachable from.
  // This creates a maximally code split bundle graph with no duplication.
  let reachable = new BitSet(assets.length);
  let reachableNonEntries = new BitSet(assets.length);
  let reachableIntersection = new BitSet(assets.length);
  for (let i = 0; i < assets.length; i++) {
    let asset = assets[i];
    let manualSharedObject = manualAssetToConfig.get(asset);

    if (bundleRoots.has(asset) && inlineConstantDeps.get(asset).size > 0) {
      let entryBundleId = nullthrows(bundleRoots.get(asset))[0];
      let entryBundle = nullthrows(bundleGraph.getNode(entryBundleId));
      invariant(entryBundle !== 'root');
      assignInlineConstants(asset, entryBundle);
    }

    if (asset.meta.isConstantModule === true) {
      // Ignore constant modules as they are placed with their direct parents
      continue;
    }

    // Unreliable bundleRoot assets which need to pulled in by shared bundles or other means.
    // Filter out entries, since they can't have shared bundles.
    // Neither can non-splittable, isolated, or needing of stable name bundles.
    // Reserve those filtered out bundles since we add the asset back into them.
    reachableNonEntries.clear();
    reachableRoots[i].forEach((nodeId) => {
      let assetId = bundleRootGraph.getNode(nodeId);
      if (assetId == null) return; // deleted
      let a = assets[assetId];
      if (
        entries.has(a) ||
        !a.isBundleSplittable ||
        (bundleRoots.get(a) &&
          (getBundleFromBundleRoot(a).needsStableName ||
            getBundleFromBundleRoot(a).bundleBehavior === 'isolated'))
      ) {
        // Add asset to non-splittable bundles.
        addAssetToBundleRoot(asset, a);
      } else if (!ancestorAssets[nodeId]?.has(i)) {
        // Filter out bundles from this asset's reachable array if
        // bundle does not contain the asset in its ancestry
        reachableNonEntries.add(assetId);
      }
    });

    reachable.bits.set(reachableNonEntries.bits);

    // If we encounter a "manual" asset, draw an edge from reachable to its MSB
    if (manualSharedObject && !reachable.empty()) {
      let bundle;
      let bundleId;
      let manualSharedBundleKey = manualSharedObject.name + ',' + asset.type;
      let sourceBundles = [];
      reachable.forEach((id) => {
        sourceBundles.push(nullthrows(bundleRoots.get(assets[id]))[0]);
      });

      if (!manualSharedMap.has(manualSharedBundleKey)) {
        let firstSourceBundle = nullthrows(
          bundleGraph.getNode(sourceBundles[0]),
        );
        invariant(firstSourceBundle !== 'root');

        bundle = createBundle({
          env: firstSourceBundle.env,
          manualSharedBundle: manualSharedObject?.name,
          sourceBundles: new Set(sourceBundles),
          target: firstSourceBundle.target,
          type: asset.type,
          uniqueKey: manualSharedBundleKey,
        });
        bundle.assets.add(asset);
        bundleId = bundleGraph.addNode(bundle);
        manualSharedMap.set(manualSharedBundleKey, bundleId);
      } else {
        bundleId = nullthrows(manualSharedMap.get(manualSharedBundleKey));
        bundle = nullthrows(bundleGraph.getNode(bundleId));
        invariant(
          bundle != null && bundle !== 'root',
          'We tried to use the root incorrectly',
        );

        if (!bundle.assets.has(asset)) {
          bundle.assets.add(asset);
          bundle.size += asset.stats.size;
        }

        for (let s of sourceBundles) {
          if (s != bundleId) {
            bundle.sourceBundles.add(s);
          }
        }
      }

      for (let sourceBundleId of sourceBundles) {
        if (bundleId !== sourceBundleId) {
          bundleGraph.addEdge(sourceBundleId, bundleId);
        }
      }

      dependencyBundleGraph.addNodeByContentKeyIfNeeded(String(bundleId), {
        value: bundle,
        type: 'bundle',
      });
      continue;
    }

    // Finally, filter out bundleRoots (bundles) from this assets
    // reachable if they are subgraphs, and reuse that subgraph bundle
    // by drawing an edge. Essentially, if two bundles within an asset's
    // reachable array, have an ancestor-subgraph relationship, draw that edge.
    // This allows for us to reuse a bundle instead of making a shared bundle if
    // a bundle represents the exact set of assets a set of bundles would share

    // if a bundle b is a subgraph of another bundle f, reuse it, drawing an edge between the two
    if (config.disableSharedBundles === false) {
      reachableNonEntries.forEach((candidateId) => {
        let candidateSourceBundleRoot = assets[candidateId];
        let candidateSourceBundleId = nullthrows(
          bundleRoots.get(candidateSourceBundleRoot),
        )[0];
        if (candidateSourceBundleRoot.env.isIsolated()) {
          return;
        }
        let reuseableBundleId = bundles.get(asset.id);
        if (reuseableBundleId != null) {
          reachable.delete(candidateId);
          bundleGraph.addEdge(candidateSourceBundleId, reuseableBundleId);

          let reusableBundle = bundleGraph.getNode(reuseableBundleId);
          invariant(reusableBundle !== 'root' && reusableBundle != null);
          reusableBundle.sourceBundles.add(candidateSourceBundleId);
        } else {
          // Asset is not a bundleRoot, but if its ancestor bundle (in the asset's reachable) can be
          // reused as a subgraph of another bundleRoot in its reachable, reuse it
          reachableIntersection.bits.set(reachableNonEntries.bits);
          reachableIntersection.intersect(
            reachableAssets[
              nullthrows(assetToBundleRootNodeId.get(candidateSourceBundleRoot))
            ],
          );

          reachableIntersection.forEach((otherCandidateId) => {
            // In the case of a circular dependency, you may end up with two
            // reusable bundles that each delete the other, leaving no reusable
            // bundles actually reachable. This check is to avoid assigning the
            // asset to a reusable bundle that has already been marked unreachable.
            if (!reachable.has(otherCandidateId)) return;

            let otherReuseCandidate = assets[otherCandidateId];
            if (candidateSourceBundleRoot === otherReuseCandidate) return;
            let reusableBundleId = nullthrows(
              bundles.get(otherReuseCandidate.id),
            );
            reachable.delete(candidateId);
            bundleGraph.addEdge(
              nullthrows(bundles.get(candidateSourceBundleRoot.id)),
              reusableBundleId,
            );
            let reusableBundle = bundleGraph.getNode(reusableBundleId);
            invariant(reusableBundle !== 'root' && reusableBundle != null);
            reusableBundle.sourceBundles.add(candidateSourceBundleId);
          });
        }
      });
    }

    let reachableArray = [];
    reachable.forEach((id) => {
      reachableArray.push(assets[id]);
    });

    // Create shared bundles for splittable bundles.
    if (
      config.disableSharedBundles === false &&
      reachableArray.length > config.minBundles
    ) {
      let sourceBundles = reachableArray.map(
        (a) => nullthrows(bundleRoots.get(a))[0],
      );
      let key = reachableArray.map((a) => a.id).join(',') + '.' + asset.type;
      let bundleId = bundles.get(key);
      let bundle;
      if (bundleId == null) {
        let firstSourceBundle = nullthrows(
          bundleGraph.getNode(sourceBundles[0]),
        );
        invariant(firstSourceBundle !== 'root');
        bundle = createBundle({
          env: firstSourceBundle.env,
          sourceBundles: new Set(sourceBundles),
          target: firstSourceBundle.target,
          type: asset.type,
        });
        let sharedInternalizedAssets = firstSourceBundle.internalizedAssets
          ? firstSourceBundle.internalizedAssets.clone()
          : new BitSet(assets.length);

        for (let p of sourceBundles) {
          let parentBundle = nullthrows(bundleGraph.getNode(p));
          invariant(parentBundle !== 'root');
          if (parentBundle === firstSourceBundle) continue;

          if (parentBundle.internalizedAssets) {
            sharedInternalizedAssets.intersect(parentBundle.internalizedAssets);
          } else {
            sharedInternalizedAssets.clear();
          }
        }
        bundle.internalizedAssets = sharedInternalizedAssets;
        bundleId = bundleGraph.addNode(bundle);
        bundles.set(key, bundleId);
      } else {
        bundle = nullthrows(bundleGraph.getNode(bundleId));
        invariant(bundle !== 'root');
      }
      bundle.assets.add(asset);
      bundle.size += asset.stats.size;

      assignInlineConstants(asset, bundle);

      for (let sourceBundleId of sourceBundles) {
        if (bundleId !== sourceBundleId) {
          bundleGraph.addEdge(sourceBundleId, bundleId);
        }
      }

      dependencyBundleGraph.addNodeByContentKeyIfNeeded(String(bundleId), {
        value: bundle,
        type: 'bundle',
      });
    } else if (
      config.disableSharedBundles === true ||
      reachableArray.length <= config.minBundles
    ) {
      for (let root of reachableArray) {
        addAssetToBundleRoot(asset, root);
      }
    }
  }

  let manualSharedBundleIds = new Set([...manualSharedMap.values()]);
  // Step split manual shared bundles for those that have the "split" property set
  let remainderMap = new DefaultMap(() => []);
  for (let id of manualSharedMap.values()) {
    let manualBundle = bundleGraph.getNode(id);
    invariant(manualBundle !== 'root' && manualBundle != null);

    if (manualBundle.sourceBundles.size > 0) {
      let firstSourceBundle = nullthrows(
        bundleGraph.getNode([...manualBundle.sourceBundles][0]),
      );
      invariant(firstSourceBundle !== 'root');
      let firstAsset = [...manualBundle.assets][0];
      let manualSharedObject = manualAssetToConfig.get(firstAsset);
      invariant(manualSharedObject != null);
      let modNum = manualAssetToConfig.get(firstAsset)?.split;
      if (modNum != null) {
        for (let a of [...manualBundle.assets]) {
          let numRep = getBigIntFromContentKey(a.id);
          // $FlowFixMe Flow doesn't know about BigInt
          let r = Number(numRep % BigInt(modNum));

          remainderMap.get(r).push(a);
        }

        for (let i = 1; i < [...remainderMap.keys()].length; i++) {
          let bundle = createBundle({
            env: firstSourceBundle.env,
            manualSharedBundle: manualSharedObject.name,
            sourceBundles: manualBundle.sourceBundles,
            target: firstSourceBundle.target,
            type: firstSourceBundle.type,
            uniqueKey: manualSharedObject.name + firstSourceBundle.type + i,
          });
          bundle.internalizedAssets = manualBundle.internalizedAssets;
          let bundleId = bundleGraph.addNode(bundle);
          manualSharedBundleIds.add(bundleId);
          for (let sourceBundleId of manualBundle.sourceBundles) {
            if (bundleId !== sourceBundleId) {
              bundleGraph.addEdge(sourceBundleId, bundleId);
            }
          }
          for (let sp of remainderMap.get(i)) {
            bundle.assets.add(sp);
            bundle.size += sp.stats.size;
            manualBundle.assets.delete(sp);
            manualBundle.size -= sp.stats.size;
          }
        }
      }
    }
  }

  // Step insert constant modules into manual shared bundles.
  // We have to do this separately as they're the only case where a single asset can
  // match multiple MSB's
  for (let [asset, msbs] of constantModuleToMSB.entries()) {
    for (let manualSharedObject of msbs) {
      let bundleId = manualSharedMap.get(manualSharedObject.name + ',js');
      if (bundleId == null) continue;
      let bundle = nullthrows(bundleGraph.getNode(bundleId));
      invariant(
        bundle != null && bundle !== 'root',
        'We tried to use the root incorrectly',
      );

      if (!bundle.assets.has(asset)) {
        bundle.assets.add(asset);
        bundle.size += asset.stats.size;
      }
    }
  }

  // Step Merge Share Bundles: Merge any shared bundles under the minimum bundle size back into
  // their source bundles, and remove the bundle.
  // We should include "bundle reuse" as shared bundles that may be removed but the bundle itself would have to be retained
  for (let [bundleNodeId, bundle] of bundleGraph.nodes.entries()) {
    if (!bundle || bundle === 'root') continue;
    if (
      bundle.sourceBundles.size > 0 &&
      bundle.mainEntryAsset == null &&
      bundle.size < config.minBundleSize &&
      !manualSharedBundleIds.has(bundleNodeId)
    ) {
      removeBundle(bundleGraph, bundleNodeId, assetReference);
    }
  }

  let modifiedSourceBundles = new Set();

  // Step Remove Shared Bundles: Remove shared bundles from bundle groups that hit the parallel request limit.
  if (config.disableSharedBundles === false) {
    for (let bundleGroupId of bundleGraph.getNodeIdsConnectedFrom(rootNodeId)) {
      // Find shared bundles in this bundle group.
      let bundleId = bundleGroupId;

      // We should include "bundle reuse" as shared bundles that may be removed but the bundle itself would have to be retained
      let bundleIdsInGroup = getBundlesForBundleGroup(bundleId); //get all bundlegrups this bundle is an ancestor of

      // Filter out inline assests as they should not contribute to PRL
      let numBundlesContributingToPRL = bundleIdsInGroup.reduce((count, b) => {
        let bundle = nullthrows(bundleGraph.getNode(b));
        invariant(bundle !== 'root');
        return count + (bundle.bundleBehavior !== 'inline');
      }, 0);

      if (numBundlesContributingToPRL > config.maxParallelRequests) {
        let sharedBundleIdsInBundleGroup = bundleIdsInGroup.filter((b) => {
          let bundle = nullthrows(bundleGraph.getNode(b));
          // shared bundles must have source bundles, we could have a bundle
          // connected to another bundle that isnt a shared bundle, so check
          return (
            bundle !== 'root' &&
            bundle.sourceBundles.size > 0 &&
            bundleId != b &&
            !manualSharedBundleIds.has(b)
          );
        });

        // Sort the bundles so the smallest ones are removed first.
        let sharedBundlesInGroup = sharedBundleIdsInBundleGroup
          .map((id) => ({
            id,
            bundle: nullthrows(bundleGraph.getNode(id)),
          }))
          .map(({id, bundle}) => {
            // For Flow
            invariant(bundle !== 'root');
            return {id, bundle};
          })
          .sort((a, b) => b.bundle.size - a.bundle.size);

        // Remove bundles until the bundle group is within the parallel request limit.
        while (
          sharedBundlesInGroup.length > 0 &&
          numBundlesContributingToPRL > config.maxParallelRequests
        ) {
          let bundleTuple = sharedBundlesInGroup.pop();
          let bundleToRemove = bundleTuple.bundle;
          let bundleIdToRemove = bundleTuple.id;
          //TODO add integration test where bundles in bunlde group > max parallel request limit & only remove a couple shared bundles
          // but total # bundles still exceeds limit due to non shared bundles

          // Add all assets in the shared bundle into the source bundles that are within this bundle group.
          let sourceBundles = [...bundleToRemove.sourceBundles].filter((b) =>
            bundleIdsInGroup.includes(b),
          );

          for (let sourceBundleId of sourceBundles) {
            let sourceBundle = nullthrows(bundleGraph.getNode(sourceBundleId));
            invariant(sourceBundle !== 'root');
            modifiedSourceBundles.add(sourceBundle);
            bundleToRemove.sourceBundles.delete(sourceBundleId);
            for (let asset of bundleToRemove.assets) {
              addAssetToBundleRoot(
                asset,
                nullthrows(sourceBundle.mainEntryAsset),
              );
            }
            //This case is specific to reused bundles, which can have shared bundles attached to it
            for (let childId of bundleGraph.getNodeIdsConnectedFrom(
              bundleIdToRemove,
            )) {
              let child = bundleGraph.getNode(childId);
              invariant(child !== 'root' && child != null);
              child.sourceBundles.add(sourceBundleId);
              bundleGraph.addEdge(sourceBundleId, childId);
            }
            // needs to add test case where shared bundle is removed from ONE bundlegroup but not from the whole graph!
            // Remove the edge from this bundle group to the shared bundle.
            // If there is now only a single bundle group that contains this bundle,
            // merge it into the remaining source bundles. If it is orphaned entirely, remove it.
            let incomingNodeCount =
              bundleGraph.getNodeIdsConnectedTo(bundleIdToRemove).length;

            if (
              incomingNodeCount <= 2 &&
              //Never fully remove reused bundles
              bundleToRemove.mainEntryAsset == null
            ) {
              // If one bundle group removes a shared bundle, but the other *can* keep it, still remove because that shared bundle is pointless (only one source bundle)
              removeBundle(bundleGraph, bundleIdToRemove, assetReference);
              // Stop iterating through bundleToRemove's sourceBundles as the bundle has been removed.
              break;
            } else {
              bundleGraph.removeEdge(sourceBundleId, bundleIdToRemove);
            }
          }
          numBundlesContributingToPRL--;
        }
      }
    }
  }

  function getBigIntFromContentKey(contentKey) {
    let b = Buffer.alloc(64);
    b.write(contentKey);
    // $FlowFixMe Flow doesn't have BigInt types in this version
    return b.readBigInt64BE();
  }
  // Fix asset order in source bundles as they are likely now incorrect after shared bundle deletion
  if (modifiedSourceBundles.size > 0) {
    let assetOrderMap = new Map(assets.map((a, index) => [a, index]));

    for (let bundle of modifiedSourceBundles) {
      bundle.assets = new Set(
        [...bundle.assets].sort((a, b) => {
          let aIndex = nullthrows(assetOrderMap.get(a));
          let bIndex = nullthrows(assetOrderMap.get(b));

          return aIndex - bIndex;
        }),
      );
    }
  }
  function deleteBundle(bundleRoot: BundleRoot) {
    bundleGraph.removeNode(nullthrows(bundles.get(bundleRoot.id)));
    bundleRoots.delete(bundleRoot);
    bundles.delete(bundleRoot.id);
    let bundleRootId = assetToBundleRootNodeId.get(bundleRoot);
    if (bundleRootId != null && bundleRootGraph.hasNode(bundleRootId)) {
      bundleRootGraph.removeNode(bundleRootId);
    }
  }
  function getBundlesForBundleGroup(bundleGroupId) {
    let bundlesInABundleGroup = [];
    bundleGraph.traverse((nodeId) => {
      bundlesInABundleGroup.push(nodeId);
    }, bundleGroupId);
    return bundlesInABundleGroup;
  }

  function getBundleFromBundleRoot(bundleRoot: BundleRoot): Bundle {
    let bundle = bundleGraph.getNode(
      nullthrows(bundleRoots.get(bundleRoot))[0],
    );
    invariant(bundle !== 'root' && bundle != null);
    return bundle;
  }

  function addAssetToBundleRoot(asset: Asset, bundleRoot: Asset) {
    let [bundleId, bundleGroupId] = nullthrows(bundleRoots.get(bundleRoot));
    let bundle = nullthrows(bundleGraph.getNode(bundleId));
    invariant(bundle !== 'root');

    if (asset.type !== bundle.type) {
      let bundleGroup = nullthrows(bundleGraph.getNode(bundleGroupId));
      invariant(bundleGroup !== 'root');
      let key = nullthrows(bundleGroup.mainEntryAsset).id + '.' + asset.type;
      let typeChangeBundleId = bundles.get(key);
      if (typeChangeBundleId == null) {
        let typeChangeBundle = createBundle({
          uniqueKey: key,
          needsStableName: bundle.needsStableName,
          bundleBehavior: bundle.bundleBehavior,
          type: asset.type,
          target: bundle.target,
          env: bundle.env,
        });
        typeChangeBundleId = bundleGraph.addNode(typeChangeBundle);
        bundleGraph.addEdge(bundleId, typeChangeBundleId);
        bundles.set(key, typeChangeBundleId);
        bundle = typeChangeBundle;
      } else {
        bundle = nullthrows(bundleGraph.getNode(typeChangeBundleId));
        invariant(bundle !== 'root');
      }
    }

    bundle.assets.add(asset);
    bundle.size += asset.stats.size;
    assignInlineConstants(asset, bundle);
  }

  function removeBundle(
    bundleGraph: IdealBundleGraph,
    bundleId: NodeId,
    assetReference: DefaultMap<Asset, Array<[Dependency, Bundle]>>,
  ) {
    let bundle = nullthrows(bundleGraph.getNode(bundleId));
    invariant(bundle !== 'root');
    for (let asset of bundle.assets) {
      assetReference.set(
        asset,
        assetReference.get(asset).filter((t) => !t.includes(bundle)),
      );
      for (let sourceBundleId of bundle.sourceBundles) {
        let sourceBundle = nullthrows(bundleGraph.getNode(sourceBundleId));
        invariant(sourceBundle !== 'root');
        addAssetToBundleRoot(asset, nullthrows(sourceBundle.mainEntryAsset));
      }
    }

    bundleGraph.removeNode(bundleId);
  }

  return {
    assets,
    bundleGraph,
    dependencyBundleGraph,
    bundleGroupBundleIds,
    assetReference,
    manualAssetToBundle,
  };
}

function createBundle(opts: {|
  asset?: Asset,
  bundleBehavior?: ?BundleBehavior,
  env?: Environment,
  manualSharedBundle?: ?string,
  needsStableName?: boolean,
  sourceBundles?: Set<NodeId>,
  target: Target,
  type?: string,
  uniqueKey?: string,
|}): Bundle {
  if (opts.asset == null) {
    return {
      assets: new Set(),
      bundleBehavior: opts.bundleBehavior,
      env: nullthrows(opts.env),
      mainEntryAsset: null,
      manualSharedBundle: opts.manualSharedBundle,
      needsStableName: Boolean(opts.needsStableName),
      size: 0,
      sourceBundles: opts.sourceBundles ?? new Set(),
      target: opts.target,
      type: nullthrows(opts.type),
      uniqueKey: opts.uniqueKey,
    };
  }

  let asset = nullthrows(opts.asset);
  return {
    assets: new Set([asset]),
    bundleBehavior: opts.bundleBehavior ?? asset.bundleBehavior,
    env: opts.env ?? asset.env,
    mainEntryAsset: asset,
    manualSharedBundle: opts.manualSharedBundle,
    needsStableName: Boolean(opts.needsStableName),
    size: asset.stats.size,
    sourceBundles: opts.sourceBundles ?? new Set(),
    target: opts.target,
    type: opts.type ?? asset.type,
    uniqueKey: opts.uniqueKey,
  };
}
