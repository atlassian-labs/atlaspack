// @flow strict-local

import {Bundler} from '@atlaspack/plugin';
import invariant from 'assert';
import {DefaultMap} from '@atlaspack/utils';
import type {
  Asset,
  Dependency,
  MutableBundleGraph,
  Bundle,
  Target,
} from '@atlaspack/types';
import {
  createPackages,
  getPackageNodes,
} from './DominatorBundler/createPackages';
import {findAssetDominators} from './DominatorBundler/findAssetDominators';
import type {
  PackageNode,
  PackagedDominatorGraph,
} from './DominatorBundler/createPackages';
import type {NodeId} from '@atlaspack/graph';
import type {AssetNode} from './DominatorBundler/bundleGraphToRootedGraph';
import type {StronglyConnectedComponentNode} from './DominatorBundler/oneCycleBreaker';
import {
  buildPackageGraph,
  buildPackageInfos,
} from './DominatorBundler/mergePackages';
import {findNodeEntryDependencies} from './DominatorBundler/findNodeEntryDependencies';
import nullthrows from 'nullthrows';

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
  // console.log('dominator bundling');
  const {dominators, graph} = findAssetDominators(bundleGraph);

  // console.log('packages');
  const packages = createPackages(bundleGraph, dominators);
  // console.log(mergedDominatorsToDot('', packages));
  // console.log('conversion');
  const {packageNodes, packageInfos} = buildPackageInfos(packages);
  const packageGraph = buildPackageGraph(
    graph,
    packages,
    packageNodes,
    packageInfos,
  );
  // console.log(mergedDominatorsToDot('', packageGraph));

  intoBundleGraph(packages, bundleGraph, packageGraph);
}

function getEntryDepForNode(
  node:
    | AssetNode
    | PackageNode
    | StronglyConnectedComponentNode<AssetNode>
    | 'root',
): Dependency | null {
  if (node === 'root') {
    return null;
  } else if (node.type === 'asset') {
    return node.entryDependency;
  } else if (node.type === 'package') {
    for (const assetNode of node.entryPointAssets) {
      return assetNode.entryDependency;
    }
  } else if (node.type === 'StronglyConnectedComponent') {
    for (const assetNode of node.values) {
      const result = getEntryDepForNode(assetNode);
      if (result) {
        return result;
      }
    }
  }
}

function getTargetForNode(
  node:
    | AssetNode
    | PackageNode
    | StronglyConnectedComponentNode<AssetNode>
    | 'root',
): Target | null {
  if (node === 'root') {
    return null;
  } else if (node.type === 'asset') {
    return node.target;
  } else if (node.type === 'package') {
    return node.entryPointAssets[0]?.target;
  } else if (node.type === 'StronglyConnectedComponent') {
    for (const assetNode of node.values) {
      const result = getEntryDepForNode(assetNode);
      if (result) {
        return result.target;
      }
    }
  }
}

interface SimpleBundle {
  entryAsset: Asset;
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
  bundleGroups: SimpleBundleGroup[];
}

export function planBundleGraph(
  packages: PackagedDominatorGraph,
  entryDependenciesByAsset: Map<
    NodeId,
    Set<{|assetNode: AssetNode, entryDependency: Dependency|}>,
  >,
  asyncDependenciesByAsset: Map<NodeId, Set<AssetNode>>,
): BundleGraphConversionResult {
  const packageNodes = getPackageNodes(packages);

  // this is not right ; we needed to connect packages to their entries and
  // treat the 'virtual' async import root packages we inserted differently

  const result = {
    bundles: [],
    bundleGroups: [],
  };

  const bundleGroups = new Map();
  const bundlesByPackageContentKey = new Map();

  for (const nodeId of packageNodes) {
    let node = packages.getNode(nodeId);
    invariant(node !== 'root');
    invariant(node != null);

    const entryDep = entryDependenciesByAsset.get(nodeId)?.values().next()
      ?.value?.entryDependency;
    console.log({entryDep, nodeId});
    invariant(entryDep != null);
    const target = entryDep.target;
    invariant(target != null);

    let bundleGroup = bundleGroups.get(entryDep);
    if (bundleGroup == null) {
      console.log('creating bundle group for', entryDep);
      bundleGroup = {entryDep, target, bundles: []};
      result.bundleGroups.push(bundleGroup);
      bundleGroups.set(entryDep, bundleGroup);
    }
    // console.log(node, entryDep, bundleGroup);

    if (node.type === 'asset') {
      // if (node.asset.type === 'js') {
      const bundle = {
        entryAsset: node.asset,
        needsStableName: node.isEntryNode,
        target,
      };
      bundlesByPackageContentKey.set(
        packages.getContentKeyByNodeId(nodeId),
        bundle,
      );

      // addNodeToBundle(packages, bundleGraph, bundle, nodeId);
      // bundleGraph.addBundleToBundleGroup(bundle, bundleGroup);
      result.bundles.push(bundle);
      bundleGroup.bundles.push(bundle);

      // } else {
      //   // TODO: handle other asset types
      // }
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

      // console.log('outputFormat', node, env.outputFormat);
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

      // addNodeToBundle(packages, bundleGraph, bundle, nodeId);
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

      // addNodeToBundle(packages, bundleGraph, bundle, nodeId);
      // bundleGraph.addBundleToBundleGroup(bundle, bundleGroup);
    } else {
      node = (node: empty);
    }
    // console.log('done', nodeId);
  }

  return result;
}

export function intoBundleGraph(
  packages: PackagedDominatorGraph,
  bundleGraph: MutableBundleGraph,
  packageGraph: PackagedDominatorGraph,
) {
  const packageNodes = getPackageNodes(packages);

  // this is not right ; we needed to connect packages to their entries and
  // treat the 'virtual' async import root packages we inserted differently

  const bundleGroups = new Map();
  const bundlesByPackageContentKey = new Map();

  for (const nodeId of packageNodes) {
    const node = packages.getNode(nodeId);
    invariant(node !== 'root');
    invariant(node != null);

    const entryDep = getEntryDepForNode(node);
    invariant(entryDep != null);
    const target = getTargetForNode(node);
    invariant(target != null);

    let bundleGroup = bundleGroups.get(entryDep);
    if (bundleGroup == null) {
      console.log('creating bundle group for', entryDep);
      bundleGroup = bundleGraph.createBundleGroup(entryDep, target);
      bundleGroups.set(entryDep, bundleGroup);
    }
    // console.log(node, entryDep, bundleGroup);

    if (node.type === 'asset') {
      // if (node.asset.type === 'js') {
      const bundle = bundleGraph.createBundle({
        entryAsset: node.asset,
        needsStableName: node.isEntryNode,
        target,
      });
      bundlesByPackageContentKey.set(
        packages.getContentKeyByNodeId(nodeId),
        bundle,
      );

      addNodeToBundle(packages, bundleGraph, bundle, nodeId);
      bundleGraph.addBundleToBundleGroup(bundle, bundleGroup);
      // } else {
      //   // TODO: handle other asset types
      // }
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

      // console.log('outputFormat', node, env.outputFormat);
      const bundle = bundleGraph.createBundle({
        env,
        type: sampleAsset.asset.type,
        uniqueKey: node.id,
        target,
      });
      bundlesByPackageContentKey.set(
        packages.getContentKeyByNodeId(nodeId),
        bundle,
      );

      addNodeToBundle(packages, bundleGraph, bundle, nodeId);
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

      const bundle = bundleGraph.createBundle({
        env,
        type: 'js',
        uniqueKey: node.id,
        target,
      });
      bundlesByPackageContentKey.set(
        packages.getContentKeyByNodeId(nodeId),
        bundle,
      );

      addNodeToBundle(packages, bundleGraph, bundle, nodeId);
      bundleGraph.addBundleToBundleGroup(bundle, bundleGroup);
    } else {
      (node: empty);
    }
    // console.log('done', nodeId);
  }

  // stitch package relations into the graph
  packageGraph.traverse((nodeId) => {
    const node = packageGraph.getNode(nodeId);
    if (node == null || node === 'root') {
      return;
    }

    const contentKey = packageGraph.getContentKeyByNodeId(nodeId);
    const bundle = bundlesByPackageContentKey.get(contentKey);
    if (bundle == null) {
      return;
    }

    const nodes = packageGraph.getNodeIdsConnectedFrom(nodeId);
    // console.log('fixing connections for', node, nodes);
    nodes.forEach((id) => {
      const child = packageGraph.getNode(id);
      if (child == null || child === 'root') {
        return;
      }

      const childContentKey = packageGraph.getContentKeyByNodeId(id);
      const childBundle = bundlesByPackageContentKey.get(childContentKey);
      if (childBundle == null) {
        return;
      }

      // console.log('connecting', node, 'to', child);
      bundleGraph.createBundleReference(bundle, childBundle);
    });
  });

  // console.log('done bundling');
}

export function addNodeToBundle(
  packages: PackagedDominatorGraph,
  bundleGraph: MutableBundleGraph,
  bundle: Bundle,
  nodeId: NodeId,
) {
  packages.traverse((id) => {
    const child = packages.getNode(id);
    if (child == null || child === 'root') {
      return;
    }

    if (child.type === 'asset') {
      bundleGraph.addAssetToBundle(child.asset, bundle);
    } else if (child.type === 'StronglyConnectedComponent') {
      for (let assetNode of child.values) {
        bundleGraph.addAssetToBundle(assetNode.asset, bundle);
      }
    }
  }, nodeId);
}

function getEntryByTarget(bundleGraph: MutableBundleGraph): {|
  targets: DefaultMap<string, Map<Asset, Dependency>>,
  allEntries: Map<Asset, Dependency>,
|} {
  // Find entries from assetGraph per target
  let targets: DefaultMap<string, Map<Asset, Dependency>> = new DefaultMap(
    () => new Map(),
  );
  const allEntries = new Map();
  bundleGraph.traverse({
    enter(node, context, actions) {
      if (node.type !== 'asset') {
        return node;
      }
      invariant(
        context != null &&
          context.type === 'dependency' &&
          context.value.isEntry &&
          context.value.target != null,
      );

      targets.get(context.value.target.distDir).set(node.value, context.value);
      allEntries.set(node.value, context.value);

      actions.skipChildren();
      return node;
    },
  });

  return {targets, allEntries};
}
