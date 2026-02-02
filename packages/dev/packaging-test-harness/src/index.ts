/* eslint-disable no-console, monorepo/no-internal-import */
import type {Bundle} from '@atlaspack/core/src/types';

import v8 from 'v8';
import path from 'path';
import fs from 'fs';
import invariant from 'assert';
import {NodeFS} from '@atlaspack/fs';
import {NodePackageManager} from '@atlaspack/package-manager';
import ThrowableDiagnostic from '@atlaspack/diagnostic';
import {initializeMonitoring} from '@atlaspack/rust';

const {
  BundleGraph: {default: BundleGraph},
  LMDBLiteCache,
  AtlaspackV3,
  FileSystemV3,
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
}

export interface PackagingResult {
  bundleId: string;
  bundleType: string;
  bundleName: string | null | undefined;
  outputPath: string;
  size: number;
  hash: string;
  cacheKeys: {
    content: string;
    map: string;
    info: string;
  };
}

/**
 * Load the BundleGraph from an Atlaspack cache directory.
 * Supports both new format (BundleGraph/ keys in LMDB) and
 * old format ({hash}-BundleGraph files in cache directory).
 */
export async function loadBundleGraphFromCache(cacheDir: string): Promise<{
  bundleGraph: InstanceType<typeof BundleGraph>;
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
    const bundleGraph = BundleGraph.deserialize(obj.bundleGraph.value);
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
  const bundleGraph = BundleGraph.deserialize(obj.bundleGraph.value);

  return {bundleGraph, cache};
}

/**
 * Get the main JS bundle from the BundleGraph.
 * If bundleFilter is provided, uses it to filter bundles.
 * Otherwise, returns the first JS bundle found.
 */
export function getMainJSBundle(
  bundleGraph: InstanceType<typeof BundleGraph>,
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
 * Run the packager on a specific bundle using AtlaspackV3.
 */
export async function packageBundle(
  bundleGraph: InstanceType<typeof BundleGraph>,
  bundle: Bundle,
  cache: InstanceType<typeof LMDBLiteCache>,
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
    corePath: path.join(__dirname, '..', '..', '..', 'core', 'core'),
    serveOptions: false,
    env: process.env,
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

    // Call package()
    const [result, error] = await atlaspackV3.package(bundle.id);

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
      cacheKeys: bundleInfo.cacheKeys,
    };
  } finally {
    atlaspackV3.end();
  }
}

/**
 * Read the packaged bundle content from LMDB cache.
 */
export async function readPackagedContent(
  cache: InstanceType<typeof LMDBLiteCache>,
  cacheKeys: {content: string; map: string; info: string},
): Promise<{content: Buffer; sourceMap: Buffer | null}> {
  // Try to read content as a large blob first, then fall back to regular blob
  let content: Buffer;
  const hasLargeBlob = await cache.hasLargeBlob(cacheKeys.content);

  if (hasLargeBlob) {
    content = await cache.getLargeBlob(cacheKeys.content);
  } else {
    content = await cache.getBlob(cacheKeys.content);
  }

  // Try to read source map if available
  let sourceMap: Buffer | null = null;
  try {
    const hasMapLargeBlob = await cache.hasLargeBlob(cacheKeys.map);
    if (hasMapLargeBlob) {
      sourceMap = await cache.getLargeBlob(cacheKeys.map);
    } else if (await cache.has(cacheKeys.map)) {
      sourceMap = await cache.getBlob(cacheKeys.map);
    }
  } catch {
    // Source map not available
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
): Promise<PackagingResult> {
  const {cacheDir, outputDir, bundleFilter, verbose} = options;

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

  // Package the bundle
  const result = await packageBundle(bundleGraph, bundle, cache, {verbose});

  // Read packaged content from cache
  const {content, sourceMap} = await readPackagedContent(
    cache,
    result.cacheKeys,
  );

  if (verbose) {
    console.log(`Packaged content size: ${content.length} bytes`);
  }

  // Write to output if outputDir is specified
  if (outputDir) {
    const outputFilename =
      bundle.name || `bundle-${bundle.id.substring(0, 8)}.${bundle.type}`;
    const outputPath = path.join(outputDir, outputFilename);

    await writePackagedContent(content, outputPath, sourceMap);

    result.outputPath = outputPath;

    if (verbose) {
      console.log(`Written to: ${outputPath}`);
      if (sourceMap) {
        console.log(`Source map written to: ${outputPath}.map`);
      }
    }
  }

  return result;
}
