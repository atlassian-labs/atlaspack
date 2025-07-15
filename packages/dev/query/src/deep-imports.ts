/* eslint-disable @typescript-eslint/no-require-imports */
/* eslint-disable monorepo/no-internal-import */
import AssetGraph from '@atlaspack/core/src/AssetGraph';
import BundleGraph, {
  bundleGraphEdgeTypes,
} from '@atlaspack/core/src/BundleGraph';
import RequestTracker, {
  RequestGraph,
  readAndDeserializeRequestGraph,
} from '@atlaspack/core/src/RequestTracker';
import {requestGraphEdgeTypes} from '@atlaspack/core/src/RequestTracker';
import {LMDBLiteCache} from '@atlaspack/cache/src/LMDBLiteCache';
import {Priority} from '@atlaspack/core/src/types';
import {fromProjectPathRelative} from '@atlaspack/core/src/projectPath';

const v =
  process.env.ATLASPACK_BUILD_ENV === 'production'
    ? {
        // Split up require specifier to outsmart packages/dev/babel-register/babel-plugin-module-translate.js
        AssetGraph: require('@atlaspack/core' + '/lib/AssetGraph').default,
        BundleGraph: require('@atlaspack/core' + '/lib/BundleGraph'),
        RequestTracker: require('@atlaspack/core' + '/lib/RequestTracker'),
        LMDBLiteCache: require('@atlaspack/cache' + '/lib/LMDBLiteCache')
          .LMDBLiteCache,
        Priority: require('@atlaspack/core' + '/lib/types').Priority,
        fromProjectPathRelative: require('@atlaspack/core' + '/lib/projectPath')
          .fromProjectPathRelative,
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

module.exports = v as {
  AssetGraph: AssetGraph;
  BundleGraph: {
    default: BundleGraph;
    bundleGraphEdgeTypes: bundleGraphEdgeTypes;
  };
  RequestTracker: {
    default: RequestTracker;
    readAndDeserializeRequestGraph: readAndDeserializeRequestGraph;
    RequestGraph: RequestGraph;
    requestGraphEdgeTypes: requestGraphEdgeTypes;
  };
  LMDBLiteCache: LMDBLiteCache;
  Priority: Priority;
  fromProjectPathRelative: fromProjectPathRelative;
};
