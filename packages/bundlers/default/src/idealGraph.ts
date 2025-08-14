import path from 'path';
import sortedArray from 'sorted-array-functions';

import {getFeatureFlag} from '@atlaspack/feature-flags';
import {
  ALL_EDGE_TYPES,
  BitSet,
  ContentGraph,
  Graph,
  NodeId,
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

import {findMergeCandidates, MergeGroup} from './bundleMerge';
import type {
  ResolvedBundlerConfig,
  SharedBundleMergeCandidates,
  AsyncBundleMerge,
} from './bundlerConfig';
import {Stats} from './stats';

/* BundleRoot - An asset that is the main entry of a Bundle. */
type BundleRoot = Asset;

export type Bundle = {
  uniqueKey: string | null | undefined;
  assets: Set<Asset>;
  internalizedAssets?: BitSet;
  bundleBehavior?: BundleBehavior | null | undefined;
  needsStableName: boolean;
  mainEntryAsset: Asset | null | undefined;
  bundleRoots: Set<Asset>;
  size: number;
  sourceBundles: Set<NodeId>;
  target: Target;
  env: Environment;
  type: string;
  manualSharedBundle: string | null | undefined; // for naming purposes;
};

export type DependencyBundleGraph = ContentGraph<
  | {
      value: Bundle;
      type: 'bundle';
    }
  | {
      value: Dependency;
      type: 'dependency';
    },
  number
>;

const dependencyPriorityEdges = {
  sync: 1,
  parallel: 2,
  lazy: 3,
  conditional: 4,
} as const;

export const idealBundleGraphEdges = Object.freeze({
  default: 1,
  conditional: 2,
});

export type IdealBundleGraph = Graph<
  Bundle | 'root',
  (typeof idealBundleGraphEdges)[keyof typeof idealBundleGraphEdges]
>;

// IdealGraph is the structure we will pass to decorate,
// which mutates the assetGraph into the bundleGraph we would
// expect from default bundler
export type IdealGraph = {
  assetReference: DefaultMap<Asset, Array<[Dependency, Bundle]>>;
  assets: Array<Asset>;
  bundleGraph: IdealBundleGraph;
  bundleGroupBundleIds: Set<NodeId>;
  dependencyBundleGraph: DependencyBundleGraph;
  manualAssetToBundle: Map<Asset, NodeId>;
};

function isNonRootBundle(
  bundle?: Bundle | 'root' | null,
  message?: string,
): Bundle {
  let existingBundle = nullthrows(bundle, message);
  invariant(existingBundle !== 'root', "Bundle cannot be 'root'");

  return existingBundle;
}

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
    Array<[Dependency, Bundle]>
  > = new DefaultMap(() => []);
  let stats = new Stats(config.projectRoot);

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
    // asset index
    number,
    (typeof bundleRootEdgeTypes)[keyof typeof bundleRootEdgeTypes]
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

  let assets: Array<Asset> = [];
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
        // @ts-expect-error TS2345
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

          // We track all matching MSB's for constant modules as they are never duplicated
          // and need to be assigned to all matching bundles
          if (node.value.meta.isConstantModule === true) {
            // @ts-expect-error TS2345
            constantModuleToMSB.get(node.value).push(c);
          }

          if (assetRegexes.some((regex) => regex.test(projectRelativePath))) {
            manualAssetToConfig.set(node.value, c);
          }
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
    Array<Asset>
  > = new DefaultMap(() => []);

  let mergeSourceBundleLookup = new Map<string, NodeId>();
  let mergeSourceBundleAssets = new Set(
    config.sharedBundleMerge?.flatMap(
      (c) =>
        c.sourceBundles?.map((assetMatch: string) =>
          path.join(config.projectRoot, assetMatch),
        ) ?? [],
    ),
  );

  /**
   * Step Create Bundles: Traverse the assetGraph (aka MutableBundleGraph) and create bundles
   * for asset type changes, parallel, inline, and async or lazy dependencies,
   * adding only that asset to each bundle, not its entire subgraph.
   */
  assetGraph.traverse(
    {
      enter(
        node: // @ts-expect-error TS2304
        | BundleGraphTraversable
          | {
              readonly type: 'dependency';
              value: Dependency;
            },
        context:
          | {
              readonly type: 'asset';
              value: Asset;
            }
          | null
          | undefined
          | {
              readonly type: 'dependency';
              value: Dependency;
            },
        // @ts-expect-error TS2304
        actions: TraversalActions,
      ) {
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
              childAsset.bundleBehavior === 'isolated' || // An isolated Dependency, or Bundle must contain all assets it needs to load.
              childAsset.bundleBehavior === 'inlineIsolated'
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
                    childAsset.bundleBehavior === 'inline' ||
                    dependency.bundleBehavior === 'inlineIsolated' ||
                    childAsset.bundleBehavior === 'inlineIsolated'
                      ? false
                      : dependency.isEntry || dependency.needsStableName,
                  target: firstBundleGroup.target,
                });
                bundleId = bundleGraph.addNode(bundle);
                bundles.set(childAsset.id, bundleId);
                bundleRoots.set(childAsset, [bundleId, bundleId]);
                bundleGroupBundleIds.add(bundleId);
                bundleGraph.addEdge(bundleGraphRootNodeId, bundleId);
                // If this asset is relevant for merging then track it's source
                // bundle id for later
                if (mergeSourceBundleAssets.has(childAsset.filePath)) {
                  mergeSourceBundleLookup.set(
                    path.relative(config.projectRoot, childAsset.filePath),
                    bundleId,
                  );
                }
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
                  (dependency.bundleBehavior === 'isolated' ||
                    dependency.bundleBehavior === 'inlineIsolated') &&
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
                // @ts-expect-error TS7053
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

                if (config.loadConditionalBundlesInParallel) {
                  // When configured, serve conditional bundles in parallel so we don't get module not found errors
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
                    dependency.bundleBehavior === 'inlineIsolated' ||
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
                  (dependency.bundleBehavior === 'isolated' ||
                    dependency.bundleBehavior === 'inlineIsolated') &&
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
              // @ts-expect-error TS2322
              bundleId = null;
            }
            if (manualSharedObject && bundleId != null) {
              // MSB Step 5:  At this point we've either created or found an existing MSB bundle
              // add the asset if it doesn't already have it and set key

              invariant(
                // @ts-expect-error TS2367
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
      // @ts-expect-error TS2322
      exit(node: BundleGraphTraversable) {
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
  let reachableRoots: Array<BitSet> = [];
  for (let i = 0; i < assets.length; i++) {
    reachableRoots.push(new BitSet(bundleRootGraph.nodes.length));
  }

  // reachableAssets is the inverse mapping of reachableRoots. For each bundle root,
  // it contains a bit set that indicates which assets are reachable from it.
  let reachableAssets: Array<BitSet> = [];

  // ancestorAssets maps bundle roots to the set of all assets available to it at runtime,
  // including in earlier parallel bundles. These are intersected through all paths to
  // the bundle to ensure that the available assets are always present no matter in which
  // order the bundles are loaded.
  let ancestorAssets: Array<null | BitSet> = [];

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
                  (config.loadConditionalBundlesInParallel &&
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
    if (
      bundleRoot.bundleBehavior === 'isolated' ||
      bundleRoot.bundleBehavior === 'inlineIsolated'
    ) {
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
      getBundleFromBundleRoot(bundleRoot).bundleBehavior !== 'isolated' &&
      getBundleFromBundleRoot(bundleRoot).bundleBehavior !== 'inlineIsolated';
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
      // @ts-expect-error TS2345
      if (!bundle.assets.has(inlineConstant)) {
        // @ts-expect-error TS2345
        bundle.assets.add(inlineConstant);
        // @ts-expect-error TS18046
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
            getBundleFromBundleRoot(a).bundleBehavior === 'isolated' ||
            getBundleFromBundleRoot(a).bundleBehavior === 'inlineIsolated'))
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
      let sourceBundles: Array<NodeId> = [];
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

    let reachableArray: Array<Asset> = [];
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
  let modifiedSourceBundles = new Set<Bundle>();

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
          let r = Number(numRep % BigInt(modNum));

          // @ts-expect-error TS2345
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
            // @ts-expect-error TS2339
            bundle.size += sp.stats.size;
            manualBundle.assets.delete(sp);
            // @ts-expect-error TS2339
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
      // @ts-expect-error TS2339
      let bundleId = manualSharedMap.get(manualSharedObject.name + ',js');
      if (bundleId == null) continue;
      let bundle = nullthrows(bundleGraph.getNode(bundleId));
      invariant(
        bundle != null && bundle !== 'root',
        'We tried to use the root incorrectly',
      );

      // @ts-expect-error TS2345
      if (!bundle.assets.has(asset)) {
        // @ts-expect-error TS2345
        bundle.assets.add(asset);
        // @ts-expect-error TS18046
        bundle.size += asset.stats.size;
      }
    }
  }

  if (getFeatureFlag('supportWebpackChunkName')) {
    // Merge webpack chunk name bundles
    let chunkNameBundles = new DefaultMap<string, Set<NodeId>>(() => new Set());
    for (let [nodeId, node] of dependencyBundleGraph.nodes.entries()) {
      // meta.chunkName is set by the Rust transformer, so we just need to find
      // bundles that have a chunkName set.
      if (
        !node ||
        node.type !== 'dependency' ||
        typeof node.value.meta.chunkName !== 'string'
      ) {
        continue;
      }

      let connectedBundles = dependencyBundleGraph.getNodeIdsConnectedFrom(
        nodeId,
        dependencyPriorityEdges[node.value.priority],
      );

      if (connectedBundles.length === 0) {
        continue;
      }

      invariant(
        connectedBundles.length === 1,
        'Expected webpackChunkName dependency to be connected to no more than one bundle',
      );

      let bundleId = connectedBundles[0];
      let bundleNode = dependencyBundleGraph.getNode(bundleId);
      invariant(bundleNode != null && bundleNode.type === 'bundle');

      // If a bundle does not have a main entry asset, it's somehow just a
      // shared bundle, and will be merged/deleted by other means.
      if (bundleNode.value.mainEntryAsset == null) {
        continue;
      }

      let bundleNodeId = null;
      let mainEntryAssetId = bundleNode.value.mainEntryAsset?.id;

      if (mainEntryAssetId != null) {
        bundleNodeId = bundles.get(mainEntryAssetId);
      }

      if (bundleNodeId == null) {
        continue;
      }

      chunkNameBundles
        .get(node.value.meta.chunkName)
        // DependencyBundleGraph uses content keys as node ids, so we can use that
        // to get the bundle id.
        .add(bundleNodeId);
    }

    for (let [chunkName, bundleIds] of chunkNameBundles.entries()) {
      // The `[request]` placeholder is not yet supported
      if (
        bundleIds.size <= 1 ||
        (typeof chunkName === 'string' && chunkName.includes('[request]'))
      ) {
        continue; // Nothing to merge
      }

      // Merge all bundles with the same chunk name into the first one.
      let [firstBundleId, ...rest] = Array.from(bundleIds);
      for (let bundleId of rest) {
        mergeBundles(firstBundleId, bundleId, 'webpack-chunk-name');
      }
    }
  }

  // Step merge async bundles that meet the configured params
  if (config.asyncBundleMerge) {
    mergeAsyncBundles(config.asyncBundleMerge);
  }

  // Step merge shared bundles that meet the configured params
  if (config.sharedBundleMerge && config.sharedBundleMerge.length > 0) {
    mergeSharedBundles(config.sharedBundleMerge);
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
        return (
          count +
          Number(
            bundle.bundleBehavior !== 'inline' &&
              bundle.bundleBehavior !== 'inlineIsolated',
          )
        );
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
          // @ts-expect-error TS18048
          let bundleToRemove = bundleTuple.bundle;
          // @ts-expect-error TS18048
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

  function mergeBundles(
    bundleToKeepId: NodeId,
    bundleToRemoveId: NodeId,
    reason: string,
  ) {
    stats.trackMerge(bundleToKeepId, bundleToRemoveId, reason);
    let bundleToKeep = isNonRootBundle(
      bundleGraph.getNode(bundleToKeepId),
      `Bundle ${bundleToKeepId} not found`,
    );
    let bundleToRemove = isNonRootBundle(
      bundleGraph.getNode(bundleToRemoveId),
      `Bundle ${bundleToRemoveId} not found`,
    );
    modifiedSourceBundles.add(bundleToKeep);
    for (let asset of bundleToRemove.assets) {
      bundleToKeep.assets.add(asset);
      bundleToKeep.size += asset.stats.size;

      let newAssetReference = assetReference
        .get(asset)
        .map(([dep, bundle]: [any, any]) =>
          bundle === bundleToRemove ? [dep, bundleToKeep] : [dep, bundle],
        );

      // @ts-expect-error TS2345
      assetReference.set(asset, newAssetReference);
    }

    // Merge any internalized assets
    if (getFeatureFlag('supportWebpackChunkName')) {
      if (bundleToKeep.internalizedAssets != null) {
        if (bundleToRemove.internalizedAssets != null) {
          bundleToKeep.internalizedAssets.intersect(
            bundleToRemove.internalizedAssets,
          );
        } else {
          bundleToKeep.internalizedAssets.clear();
        }
      }
    } else {
      invariant(
        bundleToKeep.internalizedAssets && bundleToRemove.internalizedAssets,
        'All shared bundles should have internalized assets',
      );
      bundleToKeep.internalizedAssets.union(bundleToRemove.internalizedAssets);
    }

    // Merge and clean up source bundles
    for (let sourceBundleId of bundleToRemove.sourceBundles) {
      if (bundleToKeep.sourceBundles.has(sourceBundleId)) {
        continue;
      }

      if (sourceBundleId !== bundleToKeepId) {
        bundleToKeep.sourceBundles.add(sourceBundleId);
        bundleGraph.addEdge(sourceBundleId, bundleToKeepId);
      }
    }

    if (getFeatureFlag('supportWebpackChunkName')) {
      bundleToKeep.sourceBundles.delete(bundleToRemoveId);

      for (let bundle of bundleGraph.getNodeIdsConnectedFrom(
        bundleToRemoveId,
      )) {
        let bundleNode = nullthrows(bundleGraph.getNode(bundle));
        if (bundleNode === 'root') {
          continue;
        }

        // If the bundle is a source bundle, add it to the bundle to keep
        if (bundleNode.sourceBundles.has(bundleToRemoveId)) {
          bundleNode.sourceBundles.delete(bundleToRemoveId);
          if (bundle !== bundleToKeepId) {
            bundleNode.sourceBundles.add(bundleToKeepId);
            bundleGraph.addEdge(bundleToKeepId, bundle);
          }
        }
      }

      // Merge bundle roots
      for (let bundleRoot of bundleToRemove.bundleRoots) {
        bundleToKeep.bundleRoots.add(bundleRoot);
      }

      if (bundleToRemove.mainEntryAsset != null) {
        invariant(bundleToKeep.mainEntryAsset != null);

        // Merge the bundles in bundle group
        let bundlesInRemoveBundleGroup =
          getBundlesForBundleGroup(bundleToRemoveId);

        let removedBundleSharedBundles = new Set<NodeId>();
        for (let bundleIdInGroup of bundlesInRemoveBundleGroup) {
          if (bundleIdInGroup === bundleToRemoveId) {
            continue;
          }
          bundleGraph.addEdge(bundleToKeepId, bundleIdInGroup);
          removedBundleSharedBundles.add(bundleIdInGroup);
        }

        if (getFeatureFlag('removeRedundantSharedBundles')) {
          // Merge any shared bundles that now have the same source bundles due to
          // the current bundle merge
          let sharedBundles = new DefaultMap<string, Array<NodeId>>(() => []);
          for (let bundleId of removedBundleSharedBundles) {
            let bundleNode = nullthrows(bundleGraph.getNode(bundleId));
            invariant(bundleNode !== 'root');
            if (
              bundleNode.mainEntryAsset != null ||
              bundleNode.manualSharedBundle != null
            ) {
              continue;
            }

            let key =
              Array.from(bundleNode.sourceBundles)
                .filter((sourceBundle) => sourceBundle !== bundleToRemoveId)
                .sort()
                .join(',') +
              '.' +
              bundleNode.type;

            sharedBundles.get(key).push(bundleId);
          }

          for (let sharedBundlesToMerge of sharedBundles.values()) {
            if (sharedBundlesToMerge.length > 1) {
              let [firstBundleId, ...rest] = sharedBundlesToMerge;
              for (let bundleId of rest) {
                mergeBundles(firstBundleId, bundleId, 'redundant-shared');
              }
            }
          }
        }

        // Remove old bundle group
        bundleGroupBundleIds.delete(bundleToRemoveId);

        // Clean up bundle roots
        let bundleRootToRemoveNodeId = nullthrows(
          assetToBundleRootNodeId.get(
            nullthrows(bundleToRemove.mainEntryAsset),
          ),
        );
        let bundleRootToKeepNodeId = nullthrows(
          assetToBundleRootNodeId.get(nullthrows(bundleToKeep.mainEntryAsset)),
        );

        for (let nodeId of bundleRootGraph.getNodeIdsConnectedTo(
          bundleRootToRemoveNodeId,
        )) {
          bundleRootGraph.addEdge(nodeId, bundleRootToKeepNodeId);
          bundleRootGraph.removeEdge(nodeId, bundleRootToRemoveNodeId);
        }

        for (let nodeId of bundleRootGraph.getNodeIdsConnectedFrom(
          bundleRootToRemoveNodeId,
        )) {
          bundleRootGraph.addEdge(bundleRootToKeepNodeId, nodeId);
          bundleRootGraph.removeEdge(bundleRootToRemoveNodeId, nodeId);
        }

        bundleRoots.set(nullthrows(bundleToRemove.mainEntryAsset), [
          bundleToKeepId,
          bundleToKeepId,
        ]);

        // Merge dependency bundle graph
        for (let dependencyNodeId of dependencyBundleGraph.getNodeIdsConnectedTo(
          dependencyBundleGraph.getNodeIdByContentKey(String(bundleToRemoveId)),
          ALL_EDGE_TYPES,
        )) {
          let dependencyNode = nullthrows(
            dependencyBundleGraph.getNode(dependencyNodeId),
          );
          invariant(dependencyNode.type === 'dependency');

          // Add dependency to the bundle to keep
          dependencyBundleGraph.addEdge(
            dependencyNodeId,
            dependencyBundleGraph.getNodeIdByContentKey(String(bundleToKeepId)),
            dependencyPriorityEdges[dependencyNode.value.priority],
          );
          // Remove dependency from the bundle to remove
          dependencyBundleGraph.removeEdge(
            dependencyNodeId,
            dependencyBundleGraph.getNodeIdByContentKey(
              String(bundleToRemoveId),
            ),
            dependencyPriorityEdges[dependencyNode.value.priority],
          );
        }
      }
    }

    bundleGraph.removeNode(bundleToRemoveId);
  }

  function mergeSharedBundles(mergeConfig: SharedBundleMergeCandidates) {
    // Find all shared bundles
    let sharedBundles = new Set<NodeId>();
    bundleGraph.traverse((nodeId) => {
      let bundle = bundleGraph.getNode(nodeId);

      if (!bundle) {
        throw new Error(`Unable to find bundle ${nodeId} in bundle graph`);
      }

      if (bundle === 'root') {
        return;
      }

      // Only consider JS shared bundles and non-reused bundles.
      // These count potentially be considered for merging in future but they're
      // more complicated to merge
      if (
        bundle.sourceBundles.size > 0 &&
        bundle.manualSharedBundle == null &&
        !bundle.mainEntryAsset &&
        bundle.type === 'js'
      ) {
        sharedBundles.add(nodeId);
      }
    });

    let clusters = findMergeCandidates(
      bundleGraph,
      Array.from(sharedBundles),
      mergeConfig.map(
        (config): MergeGroup => ({
          ...config,
          sourceBundles: config.sourceBundles?.map((assetMatch: string) => {
            let sourceBundleNodeId = mergeSourceBundleLookup.get(assetMatch);

            if (sourceBundleNodeId == null) {
              throw new Error(
                `Source bundle ${assetMatch} not found in merge source bundle lookup`,
              );
            }

            return sourceBundleNodeId;
          }),
        }),
      ),
    );

    let mergedBundles = new Set();

    for (let cluster of clusters) {
      let [mergeTarget, ...rest] = cluster;

      for (let bundleIdToMerge of rest) {
        mergeBundles(mergeTarget, bundleIdToMerge, 'shared-merge');
      }

      mergedBundles.add(mergeTarget);
    }

    if (getFeatureFlag('supportWebpackChunkName')) {
      return mergedBundles;
    }
  }

  function mergeAsyncBundles({
    bundleSize,
    maxOverfetchSize,
    ignore,
  }: AsyncBundleMerge) {
    let mergeCandidates = [];
    let ignoreRegexes = ignore?.map((glob) => globToRegex(glob)) ?? [];

    let isIgnored = (bundle: Bundle) => {
      if (!bundle.mainEntryAsset) {
        return false;
      }

      let mainEntryFilePath = path.relative(
        config.projectRoot,
        nullthrows(bundle.mainEntryAsset).filePath,
      );

      return ignoreRegexes.some((regex) => regex.test(mainEntryFilePath));
    };

    for (let [_bundleRootAsset, [bundleRootBundleId]] of bundleRoots) {
      let bundleRootBundle = nullthrows(
        bundleGraph.getNode(bundleRootBundleId),
      );
      invariant(bundleRootBundle !== 'root');

      if (
        bundleRootBundle.type === 'js' &&
        bundleRootBundle.bundleBehavior !== 'inline' &&
        bundleRootBundle.size <= bundleSize &&
        !isIgnored(bundleRootBundle)
      ) {
        mergeCandidates.push(bundleRootBundleId);
      }
    }

    let candidates = [];
    for (let i = 0; i < mergeCandidates.length; i++) {
      for (let j = i + 1; j < mergeCandidates.length; j++) {
        let a = mergeCandidates[i];
        let b = mergeCandidates[j];
        if (a === b) continue; // Skip self-comparison

        candidates.push(scoreAsyncMerge(a, b, maxOverfetchSize));
      }
    }

    let sortByScore = (
      a: AsyncBundleMergeCandidate,
      b: AsyncBundleMergeCandidate,
    ) => {
      let diff = a.score - b.score;
      if (diff > 0) {
        return 1;
      } else if (diff < 0) {
        return -1;
      }
      return 0;
    };

    candidates = candidates
      .filter(
        ({overfetchSize, score}) =>
          overfetchSize <= maxOverfetchSize && score > 0,
      )
      .sort(sortByScore);

    // Tracks the bundles that have been merged
    let merged = new Set<NodeId>();
    // Tracks the deleted bundles to the bundle they were merged into.
    let mergeRemap = new Map<NodeId, NodeId>();
    // Tracks the bundles that have been rescored and added back into the
    // candidates.
    let rescored = new DefaultMap<NodeId, Set<NodeId>>(() => new Set());

    do {
      let [a, b] = nullthrows(candidates.pop()).bundleIds;

      if (
        bundleGraph.hasNode(a) &&
        bundleGraph.hasNode(b) &&
        ((!merged.has(a) && !merged.has(b)) || rescored.get(a).has(b))
      ) {
        mergeRemap.set(b, a);
        merged.add(a);
        rescored.get(a).clear();

        mergeBundles(a, b, 'async-merge');
        continue;
      }

      // One or both of the bundles have been previously merged, so we'll
      // rescore and add the result back into the list of candidates.
      let getMergedBundleId = (bundleId: NodeId): NodeId | undefined => {
        let seen = new Set<NodeId>();
        while (!bundleGraph.hasNode(bundleId) && !seen.has(bundleId)) {
          seen.add(bundleId);
          bundleId = nullthrows(mergeRemap.get(bundleId));
        }

        if (!bundleGraph.hasNode(bundleId)) {
          return;
        }

        return bundleId;
      };

      // Map a and b to their merged bundle ids if they've already been merged
      let currentA = getMergedBundleId(a);
      let currentB = getMergedBundleId(b);

      if (
        !currentA ||
        !currentB ||
        // Bundles are already merged
        currentA === currentB
      ) {
        // This combiniation is not valid, so we skip it.
        continue;
      }

      let candidate = scoreAsyncMerge(currentA, currentB, maxOverfetchSize);

      if (candidate.overfetchSize <= maxOverfetchSize && candidate.score > 0) {
        sortedArray.add(candidates, candidate, sortByScore);
        rescored.get(currentA).add(currentB);
      }
    } while (candidates.length > 0);
  }

  function getBundle(bundleId: NodeId): Bundle {
    let bundle = bundleGraph.getNode(bundleId);
    if (bundle === 'root') {
      throw new Error(`Cannot access root bundle`);
    }
    if (bundle == null) {
      throw new Error(`Bundle ${bundleId} not found in bundle graph`);
    }

    return bundle;
  }

  function getBigIntFromContentKey(contentKey: string) {
    let b = Buffer.alloc(64);
    b.write(contentKey);
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
  function getBundlesForBundleGroup(bundleGroupId: NodeId) {
    let bundlesInABundleGroup: Array<NodeId> = [];
    bundleGraph.traverse((nodeId) => {
      bundlesInABundleGroup.push(nodeId);
    }, bundleGroupId);
    return bundlesInABundleGroup;
  }

  interface AsyncBundleMergeCandidate {
    overfetchSize: number;
    score: number;
    bundleIds: number[];
  }
  function scoreAsyncMerge(
    bundleAId: NodeId,
    bundleBId: NodeId,
    maxOverfetchSize: number,
  ): AsyncBundleMergeCandidate {
    let bundleGroupA = new Set(getBundlesForBundleGroup(bundleAId));
    let bundleGroupB = new Set(getBundlesForBundleGroup(bundleBId));

    let overlapSize = 0;
    let overfetchSize = 0;

    for (let bundleId of new Set([...bundleGroupA, ...bundleGroupB])) {
      let bundle = getBundle(bundleId);

      if (bundleGroupA.has(bundleId) && bundleGroupB.has(bundleId)) {
        overlapSize += bundle.size;
      } else {
        overfetchSize += bundle.size;
      }
    }

    let overlapPercent = overlapSize / (overfetchSize + overlapSize);

    let bundleAParents = getBundleParents(bundleAId);
    let bundleBParents = getBundleParents(bundleBId);

    let sharedParentOverlap = 0;
    let sharedParentMismatch = 0;

    for (let bundleId of new Set([...bundleAParents, ...bundleBParents])) {
      if (bundleAParents.has(bundleId) && bundleBParents.has(bundleId)) {
        sharedParentOverlap++;
      } else {
        sharedParentMismatch++;
      }
    }

    let overfetchScore = overfetchSize / maxOverfetchSize;
    let sharedParentPercent =
      sharedParentOverlap / (sharedParentOverlap + sharedParentMismatch);
    let score = sharedParentPercent + overlapPercent + overfetchScore;

    return {
      overfetchSize,
      score,
      bundleIds: [bundleAId, bundleBId],
    };
  }

  function getBundleParents(bundleId: NodeId): Set<NodeId> {
    let parents = new Set<NodeId>();
    let {bundleRoots} = getBundle(bundleId);

    for (let bundleRoot of bundleRoots) {
      let bundleRootNodeId = nullthrows(
        assetToBundleRootNodeId.get(bundleRoot),
      );
      for (let parentId of bundleRootGraph.getNodeIdsConnectedTo(
        bundleRootNodeId,
        ALL_EDGE_TYPES,
      )) {
        parents.add(parentId);
      }
    }
    return parents;
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

  stats.report((bundleId) => {
    let bundle = bundleGraph.getNode(bundleId);
    invariant(bundle !== 'root');
    return bundle;
  });

  return {
    assets,
    bundleGraph,
    dependencyBundleGraph,
    bundleGroupBundleIds,
    assetReference,
    manualAssetToBundle,
  };
}

function createBundle(opts: {
  asset?: Asset;
  bundleBehavior?: BundleBehavior | null | undefined;
  env?: Environment;
  manualSharedBundle?: string | null | undefined;
  needsStableName?: boolean;
  sourceBundles?: Set<NodeId>;
  target: Target;
  type?: string;
  uniqueKey?: string;
}): Bundle {
  if (opts.asset == null) {
    return {
      assets: new Set(),
      bundleBehavior: opts.bundleBehavior,
      env: nullthrows(opts.env),
      mainEntryAsset: null,
      bundleRoots: new Set(),
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
    bundleRoots: new Set([asset]),
    manualSharedBundle: opts.manualSharedBundle,
    needsStableName: Boolean(opts.needsStableName),
    size: asset.stats.size,
    sourceBundles: opts.sourceBundles ?? new Set(),
    target: opts.target,
    type: opts.type ?? asset.type,
    uniqueKey: opts.uniqueKey,
  };
}
