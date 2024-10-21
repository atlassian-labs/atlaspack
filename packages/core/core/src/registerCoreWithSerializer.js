// @flow

import {registerSerializableClass} from '@atlaspack/build-cache';
import {Graph} from '@atlaspack/graph';

import packageJson from '../package.json';

import {AtlaspackConfig} from './AtlaspackConfig';
import AssetGraph from './AssetGraph';
import BundleGraph from './BundleGraph';
import Config from './public/Config';
import {RequestGraph} from './RequestTracker';

let coreRegistered;
export function registerCoreWithSerializer() {
  if (coreRegistered) {
    return;
  }
  const packageVersion: mixed = packageJson.version;
  if (typeof packageVersion !== 'string') {
    throw new Error('Expected package version to be a string');
  }

  // $FlowFixMe[incompatible-cast]
  for (let [name, ctor] of (Object.entries({
    AssetGraph,
    Config,
    BundleGraph,
    Graph,
    AtlaspackConfig,
    RequestGraph,
    // $FlowFixMe[unclear-type]
  }): Array<[string, Class<any>]>)) {
    registerSerializableClass(packageVersion + ':' + name, ctor);
  }
  coreRegistered = true;
}
