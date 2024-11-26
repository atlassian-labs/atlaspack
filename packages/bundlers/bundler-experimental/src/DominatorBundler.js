// @flow strict-local

import {Bundler} from '@atlaspack/plugin';
import invariant from 'assert';
import type {
  Asset,
  Dependency,
  MutableBundleGraph,
  Target,
} from '@atlaspack/types';
import {
  createPackages,
  getPackageNodes,
} from './DominatorBundler/createPackages';
import {findAssetDominators} from './DominatorBundler/findAssetDominators';
import type {PackagedDominatorGraph} from './DominatorBundler/createPackages';
import type {NodeId} from '@atlaspack/graph';
import type {AssetNode} from './DominatorBundler/bundleGraphToRootedGraph';
import {
  buildPackageGraph,
  buildPackageInfos,
} from './DominatorBundler/mergePackages';
import {findNodeEntryDependencies} from './DominatorBundler/findNodeEntryDependencies';
import type {NodeEntryDependencies} from './DominatorBundler/findNodeEntryDependencies';

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
  const {dominators, graph} = findAssetDominators(bundleGraph);
  const entryDependencies = findNodeEntryDependencies(graph);
  const packages = createPackages(bundleGraph, dominators);
  const {packageNodes, packageInfos} = buildPackageInfos(packages);
  const packageGraph = buildPackageGraph(
    graph,
    packages,
    packageNodes,
    packageInfos,
  );

  intoBundleGraph(packages, bundleGraph, packageGraph, entryDependencies);
}

interface SimpleBundle {
  entryAsset: Asset;
  assets: Asset[];
  needsStableName: boolean;
  target: Target;
}

interface SimpleBundleGroup {
  entryDep: Dependency;
  target: Target;
  bundles: SimpleBundle[];
}

interface BundleGraphConversionResult {
  bundles: SimpleBundle[];
  bundlesByPackageContentKey: Map<string, SimpleBundle>;
  bundleGroups: SimpleBundleGroup[];
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

  const bundleGroups = new Map();
  const bundlesByPackageContentKey = result.bundlesByPackageContentKey;

  for (const nodeId of packageNodes) {
    let node = packages.getNode(nodeId);
    invariant(node !== 'root');
    invariant(node != null);

    const entryDep = entryDependenciesByAsset.get(nodeId)?.values().next()
      ?.value?.entryDependency;
    invariant(entryDep != null);
    const target = entryDep.target;
    invariant(target != null);
    const asyncDep = asyncDependenciesByAsset.get(nodeId)?.values().next();

    let bundleGroup = bundleGroups.get(entryDep);
    // if (bundleGroup == null) {

    if (asyncDep != null) {
      const dependency = packages.getEdgeWeight(
        packages.getNodeIdByContentKey('root'),
        packages.getNodeIdByContentKey(node.id),
      );
      invariant(dependency != null);
      bundleGroup = {entryDep: dependency, target, bundles: []};
    } else {
      bundleGroup = {entryDep, target, bundles: []};
    }

    result.bundleGroups.push(bundleGroup);
    bundleGroups.set(entryDep, bundleGroup);
    // }

    if (node.type === 'asset') {
      const bundle = {
        entryAsset: node.asset,
        needsStableName: node.isEntryNode,
        target,
        assets: [],
      };
      bundlesByPackageContentKey.set(
        packages.getContentKeyByNodeId(nodeId),
        bundle,
      );
      addNodeToBundle(packages, bundle, nodeId);

      result.bundles.push(bundle);
      bundleGroup.bundles.push(bundle);
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
        throw new Error('Could not find a sample asset to get environment for');
      }

      invariant(sampleAsset != null);
      invariant(sampleAsset.type === 'asset');
      const env = sampleAsset.asset.env;

      const bundle = {
        env,
        type: sampleAsset.asset.type,
        uniqueKey: node.id,
        target,
      };
      bundlesByPackageContentKey.set(
        packages.getContentKeyByNodeId(nodeId),
        bundle,
      );

      result.bundles.push(bundle);
      bundleGroup.bundles.push(bundle);
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
        throw new Error('Could not find a sample asset to get environment for');
      }

      invariant(sampleAsset != null);
      invariant(sampleAsset.type === 'asset');
      const env = sampleAsset.asset.env;

      const bundle = {
        env,
        type: 'js',
        uniqueKey: node.id,
        target,
      };
      bundlesByPackageContentKey.set(
        packages.getContentKeyByNodeId(nodeId),
        bundle,
      );

      result.bundles.push(bundle);
      bundleGroup.bundles.push(bundle);
      addNodeToBundle(packages, bundle, nodeId);
    } else {
      node = (node: empty);
    }
  }

  return result;
}

export function buildBundleGraph(
  plan: BundleGraphConversionResult,
  packageGraph: PackagedDominatorGraph,
  bundleGraph: MutableBundleGraph,
) {
  console.log(JSON.stringify(plan, null, 2));
  const bundlesByPlanBundle = new Map();

  for (const planGroup of plan.bundleGroups) {
    const bundleGroup = bundleGraph.createBundleGroup(
      planGroup.entryDep,
      planGroup.target,
    );

    for (let planBundle of planGroup.bundles) {
      const bundle =
        bundlesByPlanBundle.get(planBundle) ??
        bundleGraph.createBundle(planBundle);

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

    const nodes = packageGraph.getNodeIdsConnectedFrom(nodeId);
    nodes.forEach((id) => {
      const child = packageGraph.getNode(id);
      if (child == null || child === 'root') {
        return;
      }
      const childContentKey = packageGraph.getContentKeyByNodeId(id);
      const childPlanBundle =
        plan.bundlesByPackageContentKey.get(childContentKey);
      if (childPlanBundle == null) {
        return;
      }
      const childBundle = bundlesByPlanBundle.get(childPlanBundle);
      if (childBundle == null) {
        return;
      }

      bundleGraph.createBundleReference(bundle, childBundle);
    });
  });
}

export function intoBundleGraph(
  packages: PackagedDominatorGraph,
  bundleGraph: MutableBundleGraph,
  packageGraph: PackagedDominatorGraph,
  entryDependencies: NodeEntryDependencies,
) {
  const plan = planBundleGraph(
    packages,
    entryDependencies.entryDependenciesByAsset,
    entryDependencies.asyncDependenciesByAsset,
  );
  buildBundleGraph(plan, packageGraph, bundleGraph);
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
