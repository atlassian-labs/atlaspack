/**
 * Deep Imports Module
 *
 * This module provides runtime-conditional imports for internal Atlaspack modules
 * that are not publicly exported. It serves two purposes:
 *
 * 1. **Type information**: The TypeScript `import` statements at the top provide
 *    type definitions for the exported values. These are stripped at compile time.
 *
 * 2. **Runtime path switching**: The `require()` calls dynamically load modules
 *    from either `lib/` (compiled) or `src/` (source) based on environment variables:
 *    - `ATLASPACK_BUILD_ENV === 'production'` → use lib/
 *    - `ATLASPACK_REGISTER_USE_SRC === 'true'` → use src/ (with babel-register)
 *    - Otherwise → use lib/
 *
 * The string concatenation in require paths (e.g., '@atlaspack/core' + '/lib/...')
 * is intentional - it prevents babel-plugin-module-translate from rewriting these
 * paths during the build process.
 *
 * This pattern is used across Atlaspack dev tools (e.g., query, inspector) to
 * support both production builds and source-level development/debugging.
 */
/* eslint-disable monorepo/no-internal-import */

// Type-only imports - these provide TypeScript types and are stripped at runtime
import InternalBundleGraph from '@atlaspack/core/src/BundleGraph';
import {LMDBLiteCache} from '@atlaspack/cache/src/LMDBLiteCache';
import {AtlaspackV3, FileSystemV3} from '@atlaspack/core/src/atlaspack-v3';
import {DevPackager} from '@atlaspack/packager-js/src/DevPackager';
import {ScopeHoistingPackager} from '@atlaspack/packager-js/src/ScopeHoistingPackager';
import PublicBundleGraph from '@atlaspack/core/src/public/BundleGraph';
import {NamedBundle} from '@atlaspack/core/src/public/Bundle';

const v =
  process.env.ATLASPACK_BUILD_ENV === 'production' ||
  process.env.ATLASPACK_REGISTER_USE_SRC !== 'true'
    ? {
        // Split up require specifier to outsmart packages/dev/babel-register/babel-plugin-module-translate.js
        InternalBundleGraph: require('@atlaspack/core' + '/lib/BundleGraph'),
        LMDBLiteCache: require('@atlaspack/cache' + '/lib/LMDBLiteCache')
          .LMDBLiteCache,
        AtlaspackV3: require('@atlaspack/core' + '/lib/atlaspack-v3')
          .AtlaspackV3,
        FileSystemV3: require('@atlaspack/core' + '/lib/atlaspack-v3')
          .FileSystemV3,
        DevPackager: require('@atlaspack/packager-js' + '/lib/DevPackager')
          .DevPackager,
        ScopeHoistingPackager: require(
          '@atlaspack/packager-js' + '/lib/ScopeHoistingPackager',
        ).ScopeHoistingPackager,
        PublicBundleGraph: require(
          '@atlaspack/core' + '/lib/public/BundleGraph',
        ).default,
        NamedBundle: require('@atlaspack/core' + '/lib/public/Bundle')
          .NamedBundle,
      }
    : {
        InternalBundleGraph: require('@atlaspack/core/src/BundleGraph'),
        LMDBLiteCache: require('@atlaspack/cache/src/LMDBLiteCache')
          .LMDBLiteCache,
        AtlaspackV3: require('@atlaspack/core/src/atlaspack-v3').AtlaspackV3,
        FileSystemV3: require('@atlaspack/core/src/atlaspack-v3').FileSystemV3,
        DevPackager: require('@atlaspack/packager-js/src/DevPackager')
          .DevPackager,
        ScopeHoistingPackager:
          require('@atlaspack/packager-js/src/ScopeHoistingPackager')
            .ScopeHoistingPackager,
        PublicBundleGraph: require('@atlaspack/core/src/public/BundleGraph')
          .default,
        NamedBundle: require('@atlaspack/core/src/public/Bundle').NamedBundle,
      };

module.exports = v as {
  InternalBundleGraph: {
    default: typeof InternalBundleGraph;
  };
  LMDBLiteCache: typeof LMDBLiteCache;
  AtlaspackV3: typeof AtlaspackV3;
  FileSystemV3: typeof FileSystemV3;
  DevPackager: typeof DevPackager;
  ScopeHoistingPackager: typeof ScopeHoistingPackager;
  PublicBundleGraph: typeof PublicBundleGraph;
  NamedBundle: typeof NamedBundle;
};
