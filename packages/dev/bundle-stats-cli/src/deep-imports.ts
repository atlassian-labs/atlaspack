/* eslint-disable @typescript-eslint/no-require-imports */
/* eslint-disable monorepo/no-internal-import */
import {loadGraphs} from '@atlaspack/query/src/index';
import {getBundleStats} from '@atlaspack/reporter-bundle-stats/src/BundleStatsReporter';
import {PackagedBundle as PackagedBundleClass} from '@atlaspack/core/src/public/Bundle';

module.exports = (
  process.env.ATLASPACK_BUILD_ENV === 'production' ||
  process.env.ATLASPACK_REGISTER_USE_SRC !== 'true'
    ? {
        // Split up require specifier to outsmart packages/dev/babel-register/babel-plugin-module-translate.js
        loadGraphs: require('@atlaspack/query' + '/lib/index.js').loadGraphs,
        getBundleStats: require('@atlaspack/reporter-bundle-stats' +
          '/lib/BundleStatsReporter.js').getBundleStats,
        PackagedBundleClass: require('@atlaspack/core' +
          '/lib/public/Bundle.js').PackagedBundle,
      }
    : {
        loadGraphs: require('@atlaspack/query/src/index.js').loadGraphs,
        getBundleStats:
          require('@atlaspack/reporter-bundle-stats/src/BundleStatsReporter.js')
            .getBundleStats,
        PackagedBundleClass: require('@atlaspack/core/src/public/Bundle.js')
          .PackagedBundle,
      }
) as {
  loadGraphs: loadGraphs;
  getBundleStats: getBundleStats;
  PackagedBundleClass: PackagedBundleClass;
};
