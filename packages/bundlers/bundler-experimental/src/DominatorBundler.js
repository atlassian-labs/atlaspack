// @flow strict-local

import {Bundler} from '@atlaspack/plugin';
import invariant from 'assert';
import {DefaultMap} from '@atlaspack/utils';
import type {
  Asset,
  Dependency,
  MutableBundleGraph,
  Bundle,
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
  const dominators = findAssetDominators(bundleGraph);
  const packages = createPackages(bundleGraph, dominators);

  intoBundleGraph(packages, bundleGraph);
}

function intoBundleGraph(
  packages: PackagedDominatorGraph,
  bundleGraph: MutableBundleGraph,
) {
  const entryByTarget = getEntryByTarget(bundleGraph);
  const packageNodes = getPackageNodes(packages);

  // this is not right ; we needed to connect packages to their entries and
  // treat the 'virtual' async import root packages we inserted differently
  const entry = entryByTarget.values().next().value;
  invariant(entry != null);
  const entryDep = entry.values().next().value;
  invariant(entryDep != null);
  const target = entryDep.target;
  invariant(target != null);

  for (const nodeId of packageNodes) {
    const node = packages.getNode(nodeId);
    invariant(node !== 'root');
    invariant(node != null);

    if (node.type === 'asset') {
      const bundle = bundleGraph.createBundle({
        entryAsset: node.asset,
        target,
      });
      addNodeToBundle(packages, bundleGraph, bundle, nodeId, node);
    } else if (node.type === 'package') {
      const children = packages
        .getNodeIdsConnectedFrom(nodeId)
        .map((nodeId) => {
          const node = packages.getNode(nodeId);
          invariant(node != null && node !== 'root');
          return node;
        });
      const sampleAsset = children.find((node) => node?.type === 'asset');
      if (!sampleAsset) {
        console.log('could not find a sample asset to get environment for;');
        continue;
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

      addNodeToBundle(packages, bundleGraph, bundle, nodeId, node);
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
        console.log('could not find a sample asset to get environment for;');
        continue;
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

      addNodeToBundle(packages, bundleGraph, bundle, nodeId, node);
    } else {
      (node: empty);
    }
  }
}

function addNodeToBundle(
  packages: PackagedDominatorGraph,
  bundleGraph: MutableBundleGraph,
  bundle: Bundle,
  nodeId: NodeId,
  node: PackageNode | AssetNode | StronglyConnectedComponentNode<AssetNode>,
) {
  if (node.type === 'asset') {
    bundleGraph.addAssetGraphToBundle(node.asset, bundle);
    return;
  }

  if (node.type === 'package') {
    const children = packages.getNodeIdsConnectedFrom(nodeId).map((nodeId) => {
      const node = packages.getNode(nodeId);
      invariant(node != null && node !== 'root');
      return {nodeId, node};
    });
    children.forEach(({node, nodeId}) => {
      addNodeToBundle(packages, bundleGraph, bundle, nodeId, node);
    });
    return;
  }

  if (node.type === 'StronglyConnectedComponent') {
    for (let assetNode of node.values) {
      bundleGraph.addAssetToBundle(assetNode.asset, bundle);
    }
    return;
  }

  (node: empty);
}

function getEntryByTarget(
  bundleGraph: MutableBundleGraph,
): DefaultMap<string, Map<Asset, Dependency>> {
  // Find entries from assetGraph per target
  let targets: DefaultMap<string, Map<Asset, Dependency>> = new DefaultMap(
    () => new Map(),
  );
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
      actions.skipChildren();
      return node;
    },
  });
  return targets;
}
