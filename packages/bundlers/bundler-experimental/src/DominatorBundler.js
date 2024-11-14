// @flow strict-local

import {Bundler} from '@atlaspack/plugin';
import type {MutableBundleGraph} from '@atlaspack/types';
import {
  createPackages,
  getPackageNodes,
} from './DominatorBundler/createPackages';
import {findAssetDominators} from './DominatorBundler/findAssetDominators';
import type {PackagedDominatorGraph} from './DominatorBundler/createPackages';

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
}

function intoBundleGraph(
  packages: PackagedDominatorGraph,
  bundleGraph: MutableBundleGraph,
) {
  const packageNodes = getPackageNodes(packages);

  for (const packageNodeId of packageNodes) {
    const node = packages.getNode(packageNodeId);
  }
}
