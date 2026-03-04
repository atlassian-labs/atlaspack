/* eslint-disable no-console, monorepo/no-internal-import */
import type {Bundle} from '@atlaspack/core/src/types';
import type {PluginLogger} from '@atlaspack/types';

import path from 'path';
import {NodeFS} from '@atlaspack/fs';
import {NodePackageManager} from '@atlaspack/package-manager';
import {hashString} from '@atlaspack/rust';

import type {JSPackagerResult} from './types';

const {
  InternalBundleGraph: {default: InternalBundleGraph},
  LMDBLiteCache,
  DevPackager,
  ScopeHoistingPackager,
  PublicBundleGraph,
  NamedBundle: NamedBundleClass,
} = require('./deep-imports');

function createMinimalInitialOptions(
  projectRoot: string,
  cacheDir: string,
  cache: InstanceType<typeof LMDBLiteCache>,
): any {
  const inputFS = new NodeFS();
  return {
    entries: [],
    mode: 'production',
    env: process.env as Record<string, string>,
    hmrOptions: null,
    serveOptions: false,
    shouldBuildLazily: false,
    shouldAutoInstall: false,
    logLevel: 'error',
    projectRoot,
    cacheDir,
    inputFS,
    outputFS: inputFS,
    cache,
    packageManager: new NodePackageManager(inputFS, projectRoot),
    instanceId: 'packaging-test-harness',
    detailedReport: null,
    featureFlags: {atlaspackV3: true},
    defaultTargetOptions: {shouldScopeHoist: false},
  };
}

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
 * Run the JS packager (DevPackager or ScopeHoistingPackager) on a single bundle.
 */
export async function packageBundleWithJS(
  internalBundleGraph: InstanceType<typeof InternalBundleGraph>,
  internalBundle: Bundle,
  cache: InstanceType<typeof LMDBLiteCache>,
  options: {verbose?: boolean} = {},
): Promise<JSPackagerResult> {
  const projectRoot = process.cwd();
  const atlaspackOptions = createMinimalInitialOptions(
    projectRoot,
    path.join(projectRoot, '.parcel-cache'),
    cache,
  );
  const logger = createMinimalLogger();
  const parcelRequireName = 'parcelRequire' + hashString('').slice(-4);

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

  const shouldScopeHoist = publicBundle.env?.shouldScopeHoist;

  if (options.verbose) {
    console.log(
      `Running JS packager for bundle: ${internalBundle.id} (shouldScopeHoist: ${shouldScopeHoist})`,
    );
  }

  const startTime = performance.now();
  let contents: string;

  if (shouldScopeHoist) {
    const packager = new ScopeHoistingPackager(
      atlaspackOptions,
      publicBundleGraph,
      publicBundle,
      parcelRequireName,
      false,
      null,
      logger,
    );
    contents = (await packager.package()).contents;
  } else {
    const packager = new DevPackager(
      atlaspackOptions,
      publicBundleGraph,
      publicBundle,
      parcelRequireName,
      logger,
    );
    contents = (await packager.package()).contents;
  }

  const timeMs = performance.now() - startTime;

  return {
    bundleId: internalBundle.id,
    bundleType: internalBundle.type,
    bundleName: internalBundle.name,
    outputPath: '',
    size: Buffer.byteLength(contents, 'utf8'),
    timeMs,
    contents,
  };
}
