// @ts-expect-error - TS2307 - Cannot find module 'flow-to-typescript-codemod' or its corresponding type declarations.
import {Flow} from 'flow-to-typescript-codemod';
import {Graph} from '@atlaspack/graph';
import {registerSerializableClass} from './serializer';
import AssetGraph from './AssetGraph';
import BundleGraph from './BundleGraph';
import AtlaspackConfig from './AtlaspackConfig';
import {RequestGraph} from './RequestTracker';
import Config from './public/Config';
// @ts-expect-error - TS2732 - Cannot find module '../package.json'. Consider using '--resolveJsonModule' to import module with '.json' extension.
import packageJson from '../package.json';

// @ts-expect-error - TS7034 - Variable 'coreRegistered' implicitly has type 'any' in some locations where its type cannot be determined.
let coreRegistered;
export function registerCoreWithSerializer() {
  // @ts-expect-error - TS7005 - Variable 'coreRegistered' implicitly has an 'any' type.
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
