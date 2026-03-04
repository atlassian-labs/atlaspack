/* eslint-disable no-console, monorepo/no-internal-import */
import type {Bundle} from '@atlaspack/core/src/types';

import path from 'path';
import ThrowableDiagnostic from '@atlaspack/diagnostic';
import {NodeFS} from '@atlaspack/fs';
import {NodePackageManager} from '@atlaspack/package-manager';

import type {PackagingResult} from './types';

const {
  InternalBundleGraph: {default: InternalBundleGraph},
  LMDBLiteCache,
  AtlaspackV3,
} = require('./deep-imports');

/**
 * Get the main JS bundle from the BundleGraph.
 * If bundleFilter is provided, uses it to filter bundles.
 * Otherwise, returns the first JS entry bundle found.
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

  const jsBundles = bundles.filter((b: Bundle) => b.type === 'js');
  if (jsBundles.length === 0) {
    throw new Error('No JS bundles found in BundleGraph');
  }

  const entryBundle = jsBundles.find((b: Bundle) => b.mainEntryId != null);
  return entryBundle || jsBundles[0];
}

function createAtlaspackV3(
  cacheDir: string,
  cache: InstanceType<typeof LMDBLiteCache>,
): Promise<InstanceType<typeof AtlaspackV3>> {
  const projectRoot = process.cwd();
  return AtlaspackV3.create({
    cacheDir,
    corePath: path.join(__dirname, '..', '..', '..', 'core', 'core'),
    serveOptions: false,
    env: process.env as Record<string, string>,
    entries: [],
    lmdb: cache.getNativeRef(),
    packageManager: new NodePackageManager(new NodeFS(), projectRoot),
    featureFlags: {atlaspackV3: true},
    defaultTargetOptions: {shouldScopeHoist: false},
  });
}

/**
 * Run the native packager on a single bundle via AtlaspackV3.package().
 */
export async function packageBundle(
  bundleGraph: InstanceType<typeof InternalBundleGraph>,
  bundle: Bundle,
  cache: InstanceType<typeof LMDBLiteCache>,
  cacheDir: string,
  options: {verbose?: boolean} = {},
): Promise<PackagingResult> {
  if (options.verbose) {
    console.log('Creating AtlaspackV3 instance...');
  }

  const atlaspackV3 = await createAtlaspackV3(cacheDir, cache);

  try {
    if (options.verbose) {
      console.log('Loading BundleGraph into AtlaspackV3...');
    }
    await atlaspackV3.loadBundleGraph(bundleGraph);

    if (options.verbose) {
      console.log(`Packaging bundle: ${bundle.id} (type: ${bundle.type})`);
    }

    const startTime = performance.now();
    const [result, error] = await atlaspackV3.package(bundle.id);
    const timeMs = performance.now() - startTime;

    if (error) {
      throw new ThrowableDiagnostic({diagnostic: error});
    }

    const {bundleInfo} = result;
    return {
      bundleId: bundle.id,
      bundleType: bundle.type,
      bundleName: bundle.name,
      outputPath: '',
      size: bundleInfo.size,
      hash: bundleInfo.hash,
      timeMs,
      cacheKeys: bundleInfo.cacheKeys,
    };
  } finally {
    atlaspackV3.end();
  }
}
