/* eslint-disable no-console, monorepo/no-internal-import */
import type {Bundle} from '@atlaspack/core/src/types';
import type {NamedBundle, BundleGraph} from '@atlaspack/types';
import type {PluginLogger} from '@atlaspack/types';

import v8 from 'v8';
import path from 'path';
import fs from 'fs';
import invariant from 'assert';
import {NodeFS} from '@atlaspack/fs';
import {NodePackageManager} from '@atlaspack/package-manager';
import ThrowableDiagnostic from '@atlaspack/diagnostic';
import {initializeMonitoring, hashString} from '@atlaspack/rust';

const {
  InternalBundleGraph: {default: InternalBundleGraph},
  LMDBLiteCache,
  AtlaspackV3,
  FileSystemV3,
  DevPackager,
  ScopeHoistingPackager,
  PublicBundleGraph,
  NamedBundle: NamedBundleClass,
} = require('./deep-imports');

// Initialize monitoring/tracing early so RUST_LOG and ATLASPACK_TRACING_MODE work
try {
  initializeMonitoring();
} catch {
  // May fail if already initialized
}

export interface PackagingTestOptions {
  cacheDir: string;
  outputDir?: string;
  bundleFilter?: (bundle: Bundle) => boolean;
  verbose?: boolean;
  /** Enable comparison mode: run both native and JS packagers */
  compare?: boolean;
}

export interface PackagingResult {
  bundleId: string;
  bundleType: string;
  bundleName: string | null | undefined;
  outputPath: string;
  size: number;
  hash: string;
  timeMs: number;
  cacheKeys: {
    content: string;
    map: string;
    info: string;
  };
}

export interface JSPackagerResult {
  bundleId: string;
  bundleType: string;
  bundleName: string | null | undefined;
  outputPath: string;
  size: number;
  timeMs: number;
  contents: string;
}

export interface ComparisonResult {
  native: PackagingResult;
  js: JSPackagerResult;
  stats: {
    sizeDiff: number;
    sizeDiffPercent: number;
    timeDiff: number;
    timeDiffPercent: number;
    nativeFaster: boolean;
    nativeSmaller: boolean;
  };
}

/**
 * Load the BundleGraph from an Atlaspack cache directory.
 * Supports both new format (BundleGraph/ keys in LMDB) and
 * old format ({hash}-BundleGraph files in cache directory).
 */
export async function loadBundleGraphFromCache(cacheDir: string): Promise<{
  bundleGraph: InstanceType<typeof InternalBundleGraph>;
  cache: InstanceType<typeof LMDBLiteCache>;
}> {
  const cache = new LMDBLiteCache(cacheDir);

  // First, try the new format: BundleGraph/ keys in LMDB
  let bundleGraphBlob: string | null = null;
  for (const key of cache.keys()) {
    if (key.startsWith('BundleGraph/')) {
      bundleGraphBlob = key;
      break;
    }
  }

  if (bundleGraphBlob != null) {
    const file = await cache.getBlob(bundleGraphBlob);
    const obj = v8.deserialize(file);
    invariant(obj.bundleGraph != null, 'BundleGraph data is null');
    const bundleGraph = InternalBundleGraph.deserialize(obj.bundleGraph.value);
    return {bundleGraph, cache};
  }

  // Fall back to old format: {hash}-BundleGraph-{chunk} files in cache directory
  // Find the most recent BundleGraph file
  const files = await fs.promises.readdir(cacheDir);
  const bundleGraphFiles = files
    .filter((f) => f.endsWith('-BundleGraph-0'))
    .map((f) => ({
      name: f,
      key: f.replace(/-0$/, ''), // Remove chunk suffix to get the key
    }));

  if (bundleGraphFiles.length === 0) {
    throw new Error('BundleGraph not found in cache. Run a build first.');
  }

  // Get stats to find the most recent one
  const withStats = await Promise.all(
    bundleGraphFiles.map(async (f) => {
      const stat = await fs.promises.stat(path.join(cacheDir, f.name));
      return {...f, mtime: stat.mtime};
    }),
  );
  withStats.sort((a, b) => b.mtime.getTime() - a.mtime.getTime());
  const mostRecent = withStats[0];

  // Read the large blob (may be chunked)
  const chunks: Buffer[] = [];
  let chunkIndex = 0;
  // eslint-disable-next-line no-constant-condition
  while (true) {
    const chunkPath = path.join(cacheDir, `${mostRecent.key}-${chunkIndex}`);
    try {
      const chunk = await fs.promises.readFile(chunkPath);
      chunks.push(chunk);
      chunkIndex++;
    } catch {
      break;
    }
  }

  if (chunks.length === 0) {
    throw new Error('Failed to read BundleGraph chunks from cache.');
  }

  const file = Buffer.concat(chunks);
  const obj = v8.deserialize(file);
  invariant(obj.bundleGraph != null, 'BundleGraph data is null');
  const bundleGraph = InternalBundleGraph.deserialize(obj.bundleGraph.value);

  return {bundleGraph, cache};
}

/**
 * Get the main JS bundle from the BundleGraph.
 * If bundleFilter is provided, uses it to filter bundles.
 * Otherwise, returns the first JS bundle found.
 */
export function getMainJSBundle(
  bundleGraph: InstanceType<typeof InternalBundleGraph>,
  bundleFilter?: (bundle: Bundle) => boolean,
): Bundle {
  const bundles: Bundle[] = bundleGraph.getBundles();

  if (bundleFilter) {
    const filtered = bundles.filter(bundleFilter);
    if (filtered.length === 0) {
      throw new Error('No bundles match the provided filter');
    }
    return filtered[0];
  }

  // Default: find the first JS bundle
  const jsBundles = bundles.filter((b: Bundle) => b.type === 'js');
  if (jsBundles.length === 0) {
    throw new Error('No JS bundles found in BundleGraph');
  }

  // Prefer a bundle with a main entry (entry point bundle)
  const entryBundle = jsBundles.find((b: Bundle) => b.mainEntryId != null);
  return entryBundle || jsBundles[0];
}

/**
 * Run the packager on a specific bundle using AtlaspackV3 (native).
 */
export async function packageBundle(
  bundleGraph: InstanceType<typeof InternalBundleGraph>,
  bundle: Bundle,
  cache: InstanceType<typeof LMDBLiteCache>,
  cacheDir: string,
  options: {verbose?: boolean} = {},
): Promise<PackagingResult> {
  const inputFS = new NodeFS();
  const projectRoot = process.cwd();

  if (options.verbose) {
    console.log('Creating AtlaspackV3 instance...');
  }

  // Create AtlaspackV3 instance
  // We use minimal configuration since we're just packaging
  const atlaspackV3 = await AtlaspackV3.create({
    cacheDir,
    corePath: path.join(__dirname, '..', '..', '..', 'core', 'core'),
    serveOptions: false,
    env: process.env as Record<string, string>,
    // Entries are not used for packaging, but required for creation
    entries: [],
    fs: new FileSystemV3(inputFS),
    lmdb: cache.getNativeRef(),
    packageManager: new NodePackageManager(inputFS, projectRoot),
    featureFlags: {
      atlaspackV3: true,
    },
    defaultTargetOptions: {
      shouldScopeHoist: true,
    },
  });

  try {
    if (options.verbose) {
      console.log('Loading BundleGraph into AtlaspackV3...');
    }

    // Load the BundleGraph into the native code
    await atlaspackV3.loadBundleGraph(bundleGraph);

    if (options.verbose) {
      console.log(`Packaging bundle: ${bundle.id} (type: ${bundle.type})`);
    }

    // Call package() and measure time
    const startTime = performance.now();
    const [result, error] = await atlaspackV3.package(bundle.id);
    const endTime = performance.now();

    if (error) {
      throw new ThrowableDiagnostic({diagnostic: error});
    }

    const {bundleInfo} = result;

    return {
      bundleId: bundle.id,
      bundleType: bundle.type,
      bundleName: bundle.name,
      outputPath: '', // Will be set when writing to file
      size: bundleInfo.size,
      hash: bundleInfo.hash,
      timeMs: endTime - startTime,
      cacheKeys: bundleInfo.cacheKeys,
    };
  } finally {
    atlaspackV3.end();
  }
}

/**
 * Create a minimal AtlaspackOptions-like object for running the JS packager.
 * This needs to include the cache so that assets can retrieve their content.
 */
function createMinimalAtlaspackOptions(
  projectRoot: string,
  cache: InstanceType<typeof LMDBLiteCache>,
): any {
  const inputFS = new NodeFS();
  return {
    mode: 'production',
    parcelVersion: '2.0.0',
    env: process.env as Record<string, string>,
    hmrOptions: null,
    serveOptions: false,
    shouldBuildLazily: false,
    shouldAutoInstall: false,
    logLevel: 'info',
    projectRoot,
    cacheDir: path.join(projectRoot, '.parcel-cache'),
    inputFS,
    outputFS: inputFS,
    cache, // Required for assets to retrieve their content
    packageManager: new NodePackageManager(inputFS, projectRoot),
    instanceId: 'packaging-test-harness',
    detailedReport: null,
    featureFlags: {atlaspackV3: true},
  };
}

/**
 * Create a minimal logger for the JS packager.
 */
function createMinimalLogger(): PluginLogger {
  return {
    verbose: () => {},
    info: () => {},
    log: () => {},
    warn: () => {},
    error: () => {},
    progress: () => {},
  } as unknown as PluginLogger;
}

/**
 * Run the JS packager (DevPackager or ScopeHoistingPackager) on a bundle.
 * This is used for comparison with the native packager.
 */
export async function packageBundleWithJS(
  internalBundleGraph: InstanceType<typeof InternalBundleGraph>,
  internalBundle: Bundle,
  cache: InstanceType<typeof LMDBLiteCache>,
  options: {verbose?: boolean} = {},
): Promise<JSPackagerResult> {
  const projectRoot = process.cwd();
  const atlaspackOptions = createMinimalAtlaspackOptions(projectRoot, cache);
  const logger = createMinimalLogger();

  // Generate parcelRequireName similar to how the real packager does it
  const parcelRequireName = 'parcelRequire' + hashString('').slice(-4);

  // Create proper public bundle API wrappers
  // The DevPackager and ScopeHoistingPackager expect the public Bundle API
  // which has methods like traverseAssets, traverse, getMainEntry, etc.
  const publicBundle = NamedBundleClass.get(
    internalBundle,
    internalBundleGraph,
    atlaspackOptions,
  );
  const publicBundleGraph = new PublicBundleGraph(
    internalBundleGraph,
    (bundle: any, graph: any, opts: any) =>
      NamedBundleClass.get(bundle, graph, opts),
    atlaspackOptions,
  );

  // Access shouldScopeHoist from the public bundle's environment
  const shouldScopeHoist = publicBundle.env?.shouldScopeHoist;

  if (options.verbose) {
    console.log(
      `Running JS packager for bundle: ${internalBundle.id} (shouldScopeHoist: ${shouldScopeHoist})`,
    );
  }

  const startTime = performance.now();

  let contents: string;

  // Use ScopeHoistingPackager for scope hoisted bundles, DevPackager otherwise
  if (shouldScopeHoist) {
    const packager = new ScopeHoistingPackager(
      atlaspackOptions,
      publicBundleGraph,
      publicBundle,
      parcelRequireName,
      false, // unstable_asyncBundleRuntime
      null, // unstable_manualStaticBindingExports
      logger,
    );
    const result = await packager.package();
    contents = result.contents;
  } else {
    const packager = new DevPackager(
      atlaspackOptions,
      publicBundleGraph,
      publicBundle,
      parcelRequireName,
      logger,
    );
    const result = await packager.package();
    contents = result.contents;
  }

  const endTime = performance.now();

  return {
    bundleId: internalBundle.id,
    bundleType: internalBundle.type,
    bundleName: internalBundle.name,
    outputPath: '', // Will be set when writing to file
    size: Buffer.byteLength(contents, 'utf8'),
    timeMs: endTime - startTime,
    contents,
  };
}

/**
 * Calculate comparison statistics between native and JS packager results.
 */
export function calculateComparisonStats(
  native: PackagingResult,
  js: JSPackagerResult,
): ComparisonResult['stats'] {
  const sizeDiff = native.size - js.size;
  const sizeDiffPercent = js.size > 0 ? (sizeDiff / js.size) * 100 : 0;
  const timeDiff = native.timeMs - js.timeMs;
  const timeDiffPercent = js.timeMs > 0 ? (timeDiff / js.timeMs) * 100 : 0;

  return {
    sizeDiff,
    sizeDiffPercent,
    timeDiff,
    timeDiffPercent,
    nativeFaster: native.timeMs < js.timeMs,
    nativeSmaller: native.size < js.size,
  };
}

/**
 * Format a size in bytes to a human-readable string.
 */
export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(2)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

/**
 * Format comparison results for display.
 */
export function formatComparisonResults(comparison: ComparisonResult): string {
  const {native, js, stats} = comparison;

  const lines: string[] = [
    '',
    '='.repeat(60),
    'COMPARISON RESULTS',
    '='.repeat(60),
    '',
    'Native Packager (Rust):',
    `  Size: ${formatSize(native.size)}`,
    `  Time: ${native.timeMs.toFixed(2)}ms`,
    '',
    'JS Packager:',
    `  Size: ${formatSize(js.size)}`,
    `  Time: ${js.timeMs.toFixed(2)}ms`,
    '',
    '-'.repeat(60),
    'Differences:',
    `  Size: ${stats.sizeDiff >= 0 ? '+' : ''}${formatSize(stats.sizeDiff)} (${stats.sizeDiffPercent >= 0 ? '+' : ''}${stats.sizeDiffPercent.toFixed(2)}%)`,
    `  Time: ${stats.timeDiff >= 0 ? '+' : ''}${stats.timeDiff.toFixed(2)}ms (${stats.timeDiffPercent >= 0 ? '+' : ''}${stats.timeDiffPercent.toFixed(2)}%)`,
    '',
    'Summary:',
    `  ${stats.nativeFaster ? 'Native is FASTER' : 'JS is FASTER'} by ${Math.abs(stats.timeDiff).toFixed(2)}ms`,
    `  ${stats.nativeSmaller ? 'Native is SMALLER' : 'JS is SMALLER'} by ${formatSize(Math.abs(stats.sizeDiff))}`,
    '='.repeat(60),
  ];

  return lines.join('\n');
}

/**
 * Read the packaged bundle content from cache.
 * Checks the filesystem first (using the same path as LMDBLiteCache.getFileKey),
 * then falls back to LMDB. This ensures we can read content written by the native
 * packager regardless of the cachePerformanceImprovements feature flag.
 */
export async function readPackagedContent(
  cache: InstanceType<typeof LMDBLiteCache>,
  cacheKeys: {content: string; map: string; info: string},
): Promise<{content: Buffer; sourceMap: Buffer | null}> {
  let content: Buffer;
  const filePath = cache.getFileKey(cacheKeys.content);

  try {
    // Try filesystem first (native packager always writes here)
    content = await fs.promises.readFile(filePath);
  } catch {
    // Fall back to LMDB
    content = await cache.getBlob(cacheKeys.content);
  }

  // Try to read source map if available
  let sourceMap: Buffer | null = null;
  try {
    const mapFilePath = cache.getFileKey(cacheKeys.map);
    sourceMap = await fs.promises.readFile(mapFilePath);
  } catch {
    try {
      if (await cache.has(cacheKeys.map)) {
        sourceMap = await cache.getBlob(cacheKeys.map);
      }
    } catch {
      // Source map not available
    }
  }

  return {content, sourceMap};
}

/**
 * Write the packaged content to a file.
 */
export async function writePackagedContent(
  content: Buffer,
  outputPath: string,
  sourceMap?: Buffer | null,
): Promise<void> {
  const outputDir = path.dirname(outputPath);
  await fs.promises.mkdir(outputDir, {recursive: true});

  await fs.promises.writeFile(outputPath, content);

  if (sourceMap) {
    await fs.promises.writeFile(outputPath + '.map', sourceMap);
  }
}

/**
 * Main function: Load cache, package bundle, write output.
 */
export async function runPackagingTest(
  options: PackagingTestOptions,
): Promise<PackagingResult | ComparisonResult> {
  const {cacheDir, outputDir, bundleFilter, verbose, compare} = options;

  if (verbose) {
    console.log(`Loading BundleGraph from cache: ${cacheDir}`);
  }

  // Load BundleGraph from cache
  const {bundleGraph, cache} = await loadBundleGraphFromCache(cacheDir);

  if (verbose) {
    const bundles = bundleGraph.getBundles();
    console.log(`Found ${bundles.length} bundles`);
    bundles.forEach((b: Bundle) => {
      console.log(`  - ${b.id} (type: ${b.type}, name: ${b.name || 'N/A'})`);
    });
  }

  // Get the main JS bundle
  const bundle = getMainJSBundle(bundleGraph, bundleFilter);

  if (verbose) {
    console.log(`Selected bundle: ${bundle.id}`);
  }

  // Package the bundle with native packager
  if (verbose) {
    console.log('\n--- Running Native Packager ---');
  }
  const nativeResult = await packageBundle(
    bundleGraph,
    bundle,
    cache,
    cacheDir,
    {
      verbose,
    },
  );

  // Read packaged content from cache
  const {content: nativeContent, sourceMap} = await readPackagedContent(
    cache,
    nativeResult.cacheKeys,
  );

  if (verbose) {
    console.log(`Native packaged content size: ${nativeContent.length} bytes`);
  }

  // Write native output if outputDir is specified
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

  // If comparison mode is enabled, also run the JS packager
  if (compare) {
    if (verbose) {
      console.log('\n--- Running JS Packager ---');
    }

    let jsResult: JSPackagerResult;
    try {
      jsResult = await packageBundleWithJS(bundleGraph, bundle, cache, {
        verbose,
      });

      if (verbose) {
        console.log(`JS packaged content size: ${jsResult.size} bytes`);
      }

      // Write JS output if outputDir is specified
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

      // Calculate and return comparison
      const stats = calculateComparisonStats(nativeResult, jsResult);
      return {native: nativeResult, js: jsResult, stats};
    } catch (error) {
      console.error('\nJS Packager failed:', error);
      console.log('\nReturning native result only.');
      return nativeResult;
    }
  }

  return nativeResult;
}

/**
 * Type guard to check if result is a ComparisonResult.
 */
export function isComparisonResult(
  result: PackagingResult | ComparisonResult,
): result is ComparisonResult {
  return 'native' in result && 'js' in result && 'stats' in result;
}
