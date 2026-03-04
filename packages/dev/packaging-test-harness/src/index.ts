/* eslint-disable no-console, monorepo/no-internal-import */
import type {Bundle} from '@atlaspack/core/src/types';

import path from 'path';
import {initializeMonitoring} from '@atlaspack/rust';

import type {
  PackagingTestOptions,
  PackagingResult,
  ComparisonResult,
} from './types';
import {
  loadBundleGraphFromCache,
  readPackagedContent,
  writePackagedContent,
} from './cache';
import {getMainJSBundle, packageBundle} from './native-packager';
import {packageBundleWithJS} from './js-packager';
import {
  calculateComparisonStats,
  formatComparisonResults,
  formatSize,
  isComparisonResult,
} from './comparison';

// Initialize monitoring/tracing early so RUST_LOG and ATLASPACK_TRACING_MODE work
try {
  initializeMonitoring();
} catch {
  // May fail if already initialized
}

// Re-export everything so callers can import from a single entry point
export type {
  PackagingTestOptions,
  PackagingResult,
  JSPackagerResult,
  ComparisonResult,
} from './types';
export {
  loadBundleGraphFromCache,
  readPackagedContent,
  writePackagedContent,
} from './cache';
export {getMainJSBundle, packageBundle} from './native-packager';
export {packageBundleWithJS} from './js-packager';
export {
  formatSize,
  calculateComparisonStats,
  formatComparisonResults,
  isComparisonResult,
} from './comparison';

// ---------------------------------------------------------------------------
// Orchestrators
// ---------------------------------------------------------------------------

/**
 * Load cache, package a single bundle with the native packager, optionally compare
 * with the JS packager, and write outputs to disk.
 */
export async function runPackagingTest(
  options: PackagingTestOptions,
): Promise<PackagingResult | ComparisonResult> {
  const {cacheDir, outputDir, bundleFilter, verbose, compare} = options;

  if (verbose) {
    console.log(`Loading BundleGraph from cache: ${cacheDir}`);
  }

  const {bundleGraph, cache} = await loadBundleGraphFromCache(cacheDir);

  if (verbose) {
    const bundles = bundleGraph.getBundles();
    console.log(`Found ${bundles.length} bundles`);
    bundles.forEach((b: Bundle) => {
      console.log(`  - ${b.id} (type: ${b.type}, name: ${b.name || 'N/A'})`);
    });
  }

  const bundle = getMainJSBundle(bundleGraph, bundleFilter);

  if (verbose) {
    console.log(`Selected bundle: ${bundle.id}`);
    console.log('\n--- Running Native Packager ---');
  }

  const nativeResult = await packageBundle(
    bundleGraph,
    bundle,
    cache,
    cacheDir,
    {verbose},
  );

  const {content: nativeContent, sourceMap} = await readPackagedContent(
    cache,
    nativeResult.cacheKeys,
  );

  if (verbose) {
    console.log(`Native packaged content size: ${nativeContent.length} bytes`);
  }

  if (outputDir) {
    const outputFilename =
      bundle.name || `bundle-${bundle.id.substring(0, 8)}.${bundle.type}`;
    const outputPath = path.join(outputDir, 'native', outputFilename);
    await writePackagedContent(nativeContent, outputPath, sourceMap);
    nativeResult.outputPath = outputPath;
    if (verbose) {
      console.log(`Native output written to: ${outputPath}`);
    }
  }

  if (!compare) {
    return nativeResult;
  }

  if (verbose) {
    console.log('\n--- Running JS Packager ---');
  }

  try {
    const jsResult = await packageBundleWithJS(bundleGraph, bundle, cache, {
      verbose,
    });

    if (verbose) {
      console.log(`JS packaged content size: ${jsResult.size} bytes`);
    }

    if (outputDir) {
      const outputFilename =
        bundle.name || `bundle-${bundle.id.substring(0, 8)}.${bundle.type}`;
      const outputPath = path.join(outputDir, 'js', outputFilename);
      await writePackagedContent(
        Buffer.from(jsResult.contents, 'utf8'),
        outputPath,
      );
      jsResult.outputPath = outputPath;
      if (verbose) {
        console.log(`JS output written to: ${outputPath}`);
      }
    }

    return {
      native: nativeResult,
      js: jsResult,
      stats: calculateComparisonStats(nativeResult, jsResult),
    };
  } catch (error) {
    console.error('\nJS Packager failed:', error);
    console.log('\nReturning native result only.');
    return nativeResult;
  }
}
