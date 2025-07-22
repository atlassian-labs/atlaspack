import {Flow} from 'flow-to-typescript-codemod';

import {registerSerializableClass} from '@atlaspack/build-cache';
import {Graph} from '@atlaspack/graph';

import packageJson from '../package.json';

import {AtlaspackConfig} from './AtlaspackConfig';
import AssetGraph from './AssetGraph';
import BundleGraph from './BundleGraph';
import Config from './public/Config';
import {RequestGraph} from './RequestTracker';

// @ts-expect-error TS7034
let coreRegistered;
export function registerCoreWithSerializer() {
  // @ts-expect-error TS7005
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
  }) as Array<[string, Flow.Class<any>]>) {
    registerSerializableClass(packageVersion + ':' + name, ctor);
  }
  coreRegistered = true;
}
