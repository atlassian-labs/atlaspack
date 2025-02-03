// @flow strict-local

import {Bundler} from '@atlaspack/plugin';
import invariant from 'assert';
import {DefaultMap} from '@atlaspack/utils';
import type {
  Asset,
  BundleBehavior,
  Dependency,
  MutableBundleGraph,
  Target,
  Environment,
} from '@atlaspack/types';
import {
  createPackages,
  getPackageNodes,
} from './DominatorBundler/createPackages';
import {
  debugLog,
  findAssetDominators,
} from './DominatorBundler/findAssetDominators';
import type {
  PackagedDominatorGraph,
  PackageNode,
} from './DominatorBundler/createPackages';
import type {NodeId} from '@atlaspack/graph';
import type {
  AssetNode,
  BundleReferences,
} from './DominatorBundler/bundleGraphToRootedGraph';
import {
  buildPackageGraph,
  buildPackageInfos,
  // runMergePackages,
} from './DominatorBundler/mergePackages';
import {findNodeEntryDependencies} from './DominatorBundler/findNodeEntryDependencies';
import type {NodeEntryDependencies} from './DominatorBundler/findNodeEntryDependencies';
import type {StronglyConnectedComponentNode} from './DominatorBundler/cycleBreaker';

export type DominatorBundlerInput = {|
  bundleGraph: MutableBundleGraph,
|};

const DominatorBundler: Bundler = new Bundler({
  bundle({bundleGraph}) {
    dominatorBundler({
      bundleGraph,
    });
  },
  optimize() {},
});

export default DominatorBundler;

export function dominatorBundler({bundleGraph}: DominatorBundlerInput) {
  debugLog('building dominators');
  const {dominators, graph, bundleReferences} =
    findAssetDominators(bundleGraph);
  debugLog('finding entries');
  const entryDependencies = findNodeEntryDependencies(graph);
  debugLog('merging things into packagse');
  const packages = createPackages(graph, dominators);

  debugLog('merging packages together using heuristics');
  // const mergedPackages = runMergePackages(graph, packages);
  const mergedPackages = packages;

  debugLog('building package dependency graph');
  const {packageNodes, packageInfos} = buildPackageInfos(mergedPackages);

  const packageGraph = buildPackageGraph(
    graph,
    mergedPackages,
    packageNodes,
    packageInfos,
  );

  debugLog('converting into parcel API');
  intoBundleGraph(
    packages,
    bundleGraph,
    packageGraph,
    entryDependencies,
    bundleReferences,
  );
  debugLog('done bundling');
}

export type SimpleBundle =
  | {|
      type: 'entry',
      assets: Asset[],
      options: {|
        entryAsset: Asset,
        bundleBehavior: BundleBehavior | null,
        target: Target,
        needsStableName?: boolean,
      |},
    |}
  | {|
      type: 'shared',
      assets: Asset[],
      options: {|
        env: Environment,
        type: string,
        uniqueKey: string,
        target: Target,
        needsStableName?: boolean,
      |},
    |};

export interface SimpleBundleGroup {
  entryDep: Dependency;
  target: Target;
  bundles: SimpleBundle[];
}

export interface BundleGraphConversionResult {
  bundles: SimpleBundle[];
  bundlesByPackageContentKey: Map<string, SimpleBundle>;
  bundleGroups: SimpleBundleGroup[];
}

export function getOrCreateBundleGroupsForNode(
  bundleGroupsByEntryDep: DefaultMap<
    Target,
    Map<Dependency, SimpleBundleGroup>,
  >,
  packages: PackagedDominatorGraph,
  entryDependenciesByAsset: Map<NodeId, Set<AssetNode>>,
  asyncDependenciesByAsset: Map<NodeId, Set<AssetNode>>,
  nodeId: NodeId,
  node: AssetNode | PackageNode | StronglyConnectedComponentNode<AssetNode>,
): Set<SimpleBundleGroup> {
  const rootId = packages.getNodeIdByContentKey('root');
  const result: Set<SimpleBundleGroup> = new Set();

  // This node is either an entry-point or an async import
  if (node.type === 'asset' && node.isRoot) {
    if (node.isEntryNode) {
      const entryDependency = node.entryDependency;
      const target = node.target;
      invariant(entryDependency != null);
      invariant(target != null);

      const bundleGroupsMap = bundleGroupsByEntryDep.get(target);
      const existingBundleGroup = bundleGroupsMap.get(entryDependency);

      if (existingBundleGroup != null) {
        result.add(existingBundleGroup);
      } else {
        const bundleGroup = {
          entryDep: entryDependency,
          target: target,
          bundles: [],
        };
        result.add(bundleGroup);
        bundleGroupsMap.set(entryDependency, bundleGroup);
      }
    } else {
      const entries = entryDependenciesByAsset.get(nodeId);
      if (entries == null) {
        return result;
      }
      invariant(entries != null);
      for (let entry of entries) {
        invariant(entry.entryDependency != null);
        const target = entry.entryDependency.target;
        invariant(target != null);
        const bundleGroupsMap = bundleGroupsByEntryDep.get(target);

        const dependencies = packages.getEdgeWeight(rootId, nodeId);
        if (dependencies == null || dependencies.length === 0) continue;
        invariant(dependencies != null);
        invariant(dependencies.length > 0);

        for (let linkDependency of dependencies) {
          // Our graph will split async and different type dependencies into
          // new bundles. When the split is due to type, we don't want a new
          // bundle group.
          const dependency =
            linkDependency.priority === 'lazy'
              ? linkDependency
              : entry.entryDependency;
          invariant(dependency != null);
          const existingBundleGroup = bundleGroupsMap.get(dependency);

          if (existingBundleGroup != null) {
            result.add(existingBundleGroup);
          } else {
            const bundleGroup = {
              entryDep: dependency,
              target,
              bundles: [],
            };
            result.add(bundleGroup);
            bundleGroupsMap.set(dependency, bundleGroup);
          }
        }
      }
    }
  } else if (node.type === 'package') {
    for (let asset of node.entryPointAssets) {
      const nodeId = packages.getNodeIdByContentKey(asset.id);
      const childResult = getOrCreateBundleGroupsForNode(
        bundleGroupsByEntryDep,
        packages,
        entryDependenciesByAsset,
        asyncDependenciesByAsset,
        nodeId,
        asset,
      );
      for (let entry of childResult) {
        result.add(entry);
      }
    }
  } else if (node.type === 'StronglyConnectedComponent') {
    for (let asset of node.values) {
      const nodeId = packages.getNodeIdByContentKey(asset.id);
      const childResult = getOrCreateBundleGroupsForNode(
        bundleGroupsByEntryDep,
        packages,
        entryDependenciesByAsset,
        asyncDependenciesByAsset,
        nodeId,
        asset,
      );
      for (let entry of childResult) {
        result.add(entry);
      }
    }
  } else {
    // const entries = entryDependenciesByAsset.get(nodeId) ?? new Set();
    const asyncEntries = asyncDependenciesByAsset.get(nodeId) ?? new Set();
    const allEntries = Array.from(asyncEntries);
    // const allEntries = [...Array.from(entries), ...Array.from(asyncEntries)];

    for (let entry of allEntries) {
      const nodeId = packages.getNodeIdByContentKey(entry.id);
      const childResult = getOrCreateBundleGroupsForNode(
        bundleGroupsByEntryDep,
        packages,
        entryDependenciesByAsset,
        asyncDependenciesByAsset,
        nodeId,
        entry,
      );
      for (let entry of childResult) {
        result.add(entry);
      }
    }
  }

  return result;
}

export function planBundleGraph(
  packages: PackagedDominatorGraph,
  entryDependenciesByAsset: Map<NodeId, Set<AssetNode>>,
  asyncDependenciesByAsset: Map<NodeId, Set<AssetNode>>,
): BundleGraphConversionResult {
  const packageNodes = getPackageNodes(packages);

  const result = {
    bundles: [],
    bundlesByPackageContentKey: new Map(),
    bundleGroups: [],
  };

  const allBundleGroups = new Set();
  const bundleGroupsByEntryDep = new DefaultMap(() => new Map());
  const bundlesByPackageContentKey = result.bundlesByPackageContentKey;

  /// TODO : Add references logic to this function

  for (const nodeId of packageNodes) {
    let node = packages.getNode(nodeId);
    invariant(node !== 'root');
    invariant(node != null);

    const bundleGroups = Array.from(
      getOrCreateBundleGroupsForNode(
        bundleGroupsByEntryDep,
        packages,
        entryDependenciesByAsset,
        asyncDependenciesByAsset,
        nodeId,
        node,
      ),
    );
    for (let bundleGroup of bundleGroups) {
      allBundleGroups.add(bundleGroup);
    }
    if (bundleGroups.length === 0) continue;
    invariant(bundleGroups.length > 0);
    const targets = new Set(
      Array.from(bundleGroups.values()).map((group) => group.target),
    );

    for (let target of targets) {
      if (node.type === 'asset') {
        const bundle = {
          type: 'entry',
          assets: [],
          options: {
            entryAsset: node.asset,
            bundleBehavior: node.asset.bundleBehavior ?? null,
            needsStableName: node.isEntryNode,
            target,
          },
        };
        bundlesByPackageContentKey.set(
          packages.getContentKeyByNodeId(nodeId),
          bundle,
        );
        addNodeToBundle(packages, bundle, nodeId);

        result.bundles.push(bundle);
        bundleGroups.forEach((bundleGroup) => bundleGroup.bundles.push(bundle));
      } else if (node.type === 'package') {
        const children = packages
          .getNodeIdsConnectedFrom(nodeId)
          .map((nodeId) => {
            const node = packages.getNode(nodeId);
            invariant(node != null && node !== 'root');
            return node;
          });

        // this is not right
        const sampleAsset = children.find((node) => node?.type === 'asset');
        if (!sampleAsset) {
          throw new Error(
            'Could not find a sample asset to get environment for',
          );
        }

        invariant(sampleAsset.type === 'asset');
        const env = sampleAsset.asset.env;

        const bundle = {
          type: 'shared',
          assets: [],
          options: {
            env,
            type: sampleAsset.asset.type,
            uniqueKey: node.id,
            target,
            needsStableName: false,
          },
        };
        bundlesByPackageContentKey.set(
          packages.getContentKeyByNodeId(nodeId),
          bundle,
        );

        result.bundles.push(bundle);
        bundleGroups.forEach((bundleGroup) => bundleGroup.bundles.push(bundle));
        addNodeToBundle(packages, bundle, nodeId);

        // bundleGraph.addBundleToBundleGroup(bundle, bundleGroup);
      } else if (node.type === 'StronglyConnectedComponent') {
        const children = packages
          .getNodeIdsConnectedFrom(nodeId)
          .map((nodeId) => {
            const node = packages.getNode(nodeId);
            invariant(node != null && node !== 'root');
            return node;
          });
        const sampleAsset = children.find((node) => node?.type === 'asset');
        if (!sampleAsset) {
          throw new Error(
            'Could not find a sample asset to get environment for',
          );
        }

        invariant(sampleAsset != null);
        invariant(sampleAsset.type === 'asset');
        const env = sampleAsset.asset.env;

        const bundle = {
          type: 'shared',
          options: {
            env,
            type: 'js',
            uniqueKey: node.id,
            target,
            needsStableName: false,
          },
          assets: [],
        };
        bundlesByPackageContentKey.set(
          packages.getContentKeyByNodeId(nodeId),
          bundle,
        );

        result.bundles.push(bundle);
        bundleGroups.forEach((bundleGroup) => bundleGroup.bundles.push(bundle));
        addNodeToBundle(packages, bundle, nodeId);
      } else {
        node = (node: empty);
      }
    }
  }

  result.bundleGroups = Array.from(allBundleGroups);

  return result;
}

export function buildBundleGraph(
  plan: BundleGraphConversionResult,
  packageGraph: PackagedDominatorGraph,
  bundleGraph: MutableBundleGraph,
  bundleReferences: BundleReferences,
) {
  const bundlesByPlanBundle = new Map();

  for (const planGroup of plan.bundleGroups) {
    const bundleGroup = bundleGraph.createBundleGroup(
      planGroup.entryDep,
      planGroup.target,
    );

    for (let planBundle of planGroup.bundles) {
      const bundle =
        bundlesByPlanBundle.get(planBundle) ??
        bundleGraph.createBundle(planBundle.options);

      bundleGraph.addBundleToBundleGroup(bundle, bundleGroup);
      bundlesByPlanBundle.set(planBundle, bundle);

      for (let asset of planBundle.assets) {
        bundleGraph.addAssetToBundle(asset, bundle);
      }
    }
  }

  packageGraph.traverse((nodeId) => {
    const node = packageGraph.getNode(nodeId);
    if (node == null || node === 'root') {
      return;
    }

    const contentKey = packageGraph.getContentKeyByNodeId(nodeId);
    const planBundle = plan.bundlesByPackageContentKey.get(contentKey);
    if (planBundle == null) {
      return;
    }
    const bundle = bundlesByPlanBundle.get(planBundle);
    if (bundle == null) {
      return;
    }

    const references = bundleReferences.get(contentKey) ?? [];
    for (let {assetContentKey: childContentKey} of references) {
      const childPlanBundle =
        plan.bundlesByPackageContentKey.get(childContentKey);
      if (childPlanBundle == null) {
        continue;
      }
      const childBundle = bundlesByPlanBundle.get(childPlanBundle);
      if (childBundle == null) {
        continue;
      }

      bundleGraph.createBundleReference(bundle, childBundle);
    }
  });
}

export function intoBundleGraph(
  packages: PackagedDominatorGraph,
  bundleGraph: MutableBundleGraph,
  packageGraph: PackagedDominatorGraph,
  entryDependencies: NodeEntryDependencies,
  bundleReferences: BundleReferences,
) {
  const plan = planBundleGraph(
    packages,
    entryDependencies.entryDependenciesByAsset,
    entryDependencies.asyncDependenciesByAsset,
  );
  buildBundleGraph(plan, packageGraph, bundleGraph, bundleReferences);
}

export function addNodeToBundle(
  packages: PackagedDominatorGraph,
  bundle: SimpleBundle,
  nodeId: NodeId,
) {
  packages.traverse((id) => {
    const child = packages.getNode(id);
    if (child == null || child === 'root') {
      return;
    }

    if (child.type === 'asset') {
      bundle.assets.push(child.asset);
    } else if (child.type === 'StronglyConnectedComponent') {
      for (let assetNode of child.values) {
        bundle.assets.push(assetNode.asset);
      }
    }
  }, nodeId);
}
