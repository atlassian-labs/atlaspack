// @flow
import {Graph} from '@atlaspack/graph';
import {registerSerializableClass} from './serializer';
import AssetGraph from './AssetGraph';
import BundleGraph from './BundleGraph';
import {AtlaspackConfig} from './AtlaspackConfig';
import {RequestGraph} from './RequestTracker';
import Config from './public/Config';
import packageJson from '../package.json';

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
