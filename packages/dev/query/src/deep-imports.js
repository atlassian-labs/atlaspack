// @flow
/* eslint-disable monorepo/no-internal-import */
import typeof AssetGraph from '@atlaspack/core/src/AssetGraph';
import typeof BundleGraph, {
  bundleGraphEdgeTypes,
} from '@atlaspack/core/src/BundleGraph';
import typeof RequestTracker, {
  RequestGraph,
  readAndDeserializeRequestGraph,
} from '@atlaspack/core/src/RequestTracker';
import typeof {requestGraphEdgeTypes} from '@atlaspack/core/src/RequestTracker';
import typeof {LMDBLiteCache} from '@atlaspack/cache/src/LMDBLiteCache';
import typeof {Priority} from '@atlaspack/core/src/types';
import typeof {fromProjectPathRelative} from '@atlaspack/core/src/projectPath';

const v =
  process.env.ATLASPACK_BUILD_ENV === 'production'
    ? {
        // Split up require specifier to outsmart packages/dev/babel-register/babel-plugin-module-translate.js
        // $FlowFixMe(unsupported-syntax)
        AssetGraph: require('@atlaspack/core' + '/lib/AssetGraph.js').default,
        // $FlowFixMe(unsupported-syntax)
        BundleGraph: require('@atlaspack/core' + '/lib/BundleGraph.js'),
        // $FlowFixMe(unsupported-syntax)
        RequestTracker: require('@atlaspack/core' + '/lib/RequestTracker.js'),
        // $FlowFixMe(unsupported-syntax)
        LMDBLiteCache: require('@atlaspack/cache' + '/lib/LMDBLiteCache.js')
          .LMDBLiteCache,
        // $FlowFixMe(unsupported-syntax)
        Priority: require('@atlaspack/core' + '/lib/types.js').Priority,
        // $FlowFixMe(unsupported-syntax)
        fromProjectPathRelative: require('@atlaspack/core' +
          '/lib/projectPath.js').fromProjectPathRelative,
      }
    : {
        AssetGraph: require('@atlaspack/core/src/AssetGraph').default,
        BundleGraph: require('@atlaspack/core/src/BundleGraph'),
        RequestTracker: require('@atlaspack/core/src/RequestTracker'),
        LMDBLiteCache: require('@atlaspack/cache/src/LMDBLiteCache')
          .LMDBLiteCache,
        Priority: require('@atlaspack/core/src/types').Priority,
        fromProjectPathRelative: require('@atlaspack/core/src/projectPath')
          .fromProjectPathRelative,
      };

module.exports = (v: {|
  AssetGraph: AssetGraph,
  BundleGraph: {
    default: BundleGraph,
    bundleGraphEdgeTypes: bundleGraphEdgeTypes,
    ...
  },
  RequestTracker: {
    default: RequestTracker,
    readAndDeserializeRequestGraph: readAndDeserializeRequestGraph,
    RequestGraph: RequestGraph,
    requestGraphEdgeTypes: requestGraphEdgeTypes,
    ...
  },
  LMDBLiteCache: LMDBLiteCache,
  Priority: Priority,
  fromProjectPathRelative: fromProjectPathRelative,
|});
