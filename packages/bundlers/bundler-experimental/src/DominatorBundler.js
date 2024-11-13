// @flow strict-local

import type {BundleGraph, Bundle} from '@atlaspack/types';
import type {MutableBundleGraph} from '@atlaspack/types';
import {createPackages} from './DominatorBundler/createPackages';
import {findAssetDominators} from './DominatorBundler/findAssetDominators';

export type DominatorBundlerInput = {|
  inputGraph: MutableBundleGraph,
  outputGraph: MutableBundleGraph,
|};

export type DominatorBundlerOutput = {|
  bundleGraph: BundleGraph<Bundle>,
|};

export function dominatorBundler({
  inputGraph,
  outputGraph,
}: DominatorBundlerInput): DominatorBundlerOutput {
  const dominators = findAssetDominators(inputGraph);
  // eslint-disable-next-line no-unused-vars
  const packages = createPackages(inputGraph, dominators);

  return {
    bundleGraph: outputGraph,
  };
}
