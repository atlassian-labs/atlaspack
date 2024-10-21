import {Flow} from 'flow-to-typescript-codemod';

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
  const packageVersion: unknown = packageJson.version;
  if (typeof packageVersion !== 'string') {
    throw new Error('Expected package version to be a string');
  }

  for (let [name, ctor] of Object.entries({
    AssetGraph,
    Config,
    BundleGraph,
    Graph,
    AtlaspackConfig,
    RequestGraph,
    // $FlowFixMe[unclear-type]
  }) as Array<[string, Flow.Class<any>]>) {
    registerSerializableClass(packageVersion + ':' + name, ctor);
  }
  coreRegistered = true;
}
