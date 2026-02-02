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
import BundleGraph from '@atlaspack/core/src/BundleGraph';
import {LMDBLiteCache} from '@atlaspack/cache/src/LMDBLiteCache';
import {AtlaspackV3, FileSystemV3} from '@atlaspack/core/src/atlaspack-v3';

const v =
  process.env.ATLASPACK_BUILD_ENV === 'production' ||
  process.env.ATLASPACK_REGISTER_USE_SRC !== 'true'
    ? {
        // Split up require specifier to outsmart packages/dev/babel-register/babel-plugin-module-translate.js
        BundleGraph: require('@atlaspack/core' + '/lib/BundleGraph'),
        LMDBLiteCache: require('@atlaspack/cache' + '/lib/LMDBLiteCache')
          .LMDBLiteCache,
        AtlaspackV3: require('@atlaspack/core' + '/lib/atlaspack-v3')
          .AtlaspackV3,
        FileSystemV3: require('@atlaspack/core' + '/lib/atlaspack-v3')
          .FileSystemV3,
      }
    : {
        BundleGraph: require('@atlaspack/core/src/BundleGraph'),
        LMDBLiteCache: require('@atlaspack/cache/src/LMDBLiteCache')
          .LMDBLiteCache,
        AtlaspackV3: require('@atlaspack/core/src/atlaspack-v3').AtlaspackV3,
        FileSystemV3: require('@atlaspack/core/src/atlaspack-v3').FileSystemV3,
      };

module.exports = v as {
  BundleGraph: {
    default: typeof BundleGraph;
  };
  LMDBLiteCache: typeof LMDBLiteCache;
  AtlaspackV3: typeof AtlaspackV3;
  FileSystemV3: typeof FileSystemV3;
};
