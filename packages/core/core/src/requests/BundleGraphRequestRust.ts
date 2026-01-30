import type {Async} from '@atlaspack/types';
import type {SharedReference} from '@atlaspack/workers';
import type {AtlaspackConfig, LoadedPlugin} from '../AtlaspackConfig';
import type {StaticRunOpts, RunAPI} from '../RequestTracker';
import type {
  Asset,
  AssetGroup,
  Config,
  DevDepRequest,
  AtlaspackOptions,
  DevDepRequestRef,
} from '../types';

import nullthrows from 'nullthrows';
import {instrumentAsync} from '@atlaspack/logger';
import ThrowableDiagnostic from '@atlaspack/diagnostic';
import AssetGraph from '../AssetGraph';
import InternalBundleGraph, {bundleGraphEdgeTypes} from '../BundleGraph';
import dumpGraphToGraphViz from '../dumpGraphToGraphViz';
import {hashString} from '@atlaspack/rust';
import PluginOptions from '../public/PluginOptions';
import applyRuntimes from '../applyRuntimes';
import {ATLASPACK_VERSION} from '../constants';
import {optionsProxy} from '../utils';
import {
  createDevDependency,
  getDevDepRequests,
  invalidateDevDeps,
} from './DevDepRequest';
import {PluginWithLoadConfig} from './ConfigRequest';
import {getFeatureFlag} from '@atlaspack/feature-flags';
import {requestTypes} from '../RequestTracker';
import type {AtlaspackV3} from '../atlaspack-v3';
import createAssetGraphRequestJS from './AssetGraphRequest';
import {createAssetGraphRequestRust} from './AssetGraphRequestRust';
import type {BundleGraphResult} from './BundleGraphRequest';
import createAtlaspackConfigRequest, {
  getCachedAtlaspackConfig,
  ConfigAndCachePath,
} from './AtlaspackConfigRequest';
import {
  validateBundles,
  nameBundle,
  loadPluginConfigWithDevDeps,
  runDevDepRequest,
} from './BundleGraphRequestUtils';

type BundleGraphRequestInput = {
  requestedAssetIds: Set<string>;
  signal?: AbortSignal;
  optionsRef: SharedReference;
};

type BundleGraphRequestRustInput = {
  assetGraph: AssetGraph;
  changedAssets: Map<string, Asset>;
  assetRequests: Array<AssetGroup>;
  optionsRef: SharedReference;
  rustAtlaspack: AtlaspackV3;
};

type RunInput = {
  input: BundleGraphRequestRustInput;
} & StaticRunOpts<BundleGraphResult>;

type FactoryRunInput = {
  input: BundleGraphRequestInput;
} & StaticRunOpts<BundleGraphResult>;

type BundleGraphRequestRust = {
  id: string;
  readonly type: typeof requestTypes.bundle_graph_request;
  run: (arg1: FactoryRunInput) => Async<BundleGraphResult>;
  input: BundleGraphRequestInput;
};

/**
 * Creates a bundle graph request implementation that performs bundling via Rust.
 *
 * Note: Asset graph creation is still delegated to the existing AssetGraphRequest
 * infrastructure (JS by default, or Rust when `atlaspackV3` is enabled).
 */
export default function createBundleGraphRequestRust(
  input: BundleGraphRequestInput,
): BundleGraphRequestRust {
  return {
    type: requestTypes.bundle_graph_request,
    id: 'BundleGraphRust',
    run: async (runInput) => {
      const {options, api} = runInput;
      const {optionsRef, requestedAssetIds} = runInput.input;

      if (!runInput.rustAtlaspack) {
        throw new Error(
          'BundleGraphRequestRust requires rustAtlaspack to be present',
        );
      }

      const createAssetGraphRequest =
        getFeatureFlag('atlaspackV3') && runInput.rustAtlaspack
          ? createAssetGraphRequestRust(runInput.rustAtlaspack)
          : createAssetGraphRequestJS;

      const assetGraphRequest = createAssetGraphRequest({
        name: 'Main',
        entries: options.entries,
        optionsRef,
        shouldBuildLazily: options.shouldBuildLazily,
        lazyIncludes: options.lazyIncludes,
        lazyExcludes: options.lazyExcludes,
        requestedAssetIds,
      });

      const {assetGraph, changedAssets, assetRequests} = await api.runRequest(
        assetGraphRequest,
        {
          force:
            Boolean(runInput.rustAtlaspack) ||
            (options.shouldBuildLazily && requestedAssetIds.size > 0),
        },
      );

      return runBundleGraphRequestRust({
        ...(runInput as any),
        input: {
          assetGraph,
          changedAssets,
          assetRequests,
          optionsRef,
          rustAtlaspack: runInput.rustAtlaspack,
        },
      });
    },
    input,
  };
}

/**
 * Runs the native Rust bundler, then performs naming and runtime application in JS.
 *
 * This is the entry point for native bundling (milestone 1). The Rust side currently
 * returns a stub bundle graph, which will be expanded as the native bundling
 * implementation progresses.
 */
export async function runBundleGraphRequestRust(
  runInput: RunInput,
): Promise<BundleGraphResult> { 
  const {input, api, options} = runInput;
  const {
    assetGraph,
    changedAssets,
    assetRequests,
    optionsRef,
    rustAtlaspack,
  } = input;

  // Call the Rust bundler
  const [bundleGraphResult, bundleGraphError] = (await rustAtlaspack.buildBundleGraph()) as [
    {
      nodeCount: number;
      edges: [number, number][];
      publicIdByAssetId: {[id: string]: string};
      assetPublicIds: string[];
    },
    Error | null,
  ];

  if (bundleGraphError) {
    throw new ThrowableDiagnostic({
      diagnostic: bundleGraphError,
    });
  }

  // For now, the Rust bundler returns an empty result, so we fall back to
  // creating the bundle graph from the asset graph using JS.
  // This will be replaced once native bundling is fully implemented.
  const publicIdByAssetId = new Map(
    Object.entries(bundleGraphResult.publicIdByAssetId ?? {}),
  );
  const assetPublicIds = new Set(bundleGraphResult.assetPublicIds ?? []);

  const internalBundleGraph = InternalBundleGraph.fromAssetGraph(
    assetGraph,
    options.mode === 'production',
    publicIdByAssetId,
    assetPublicIds,
  );

  // Set up the bundler runner for naming and runtimes
  const runner = new NativeBundlerRunner(
    runInput,
    optionsRef,
  );

  await runner.loadConfigs();

  // Name all bundles
  const namers = await runner.config.getNamers();
  const bundles = internalBundleGraph.getBundles({includeInline: true});
  await Promise.all(
    bundles.map((bundle) =>
      nameBundle(
        namers,
        bundle,
        internalBundleGraph,
        options,
        runner.pluginOptions,
        runner.configs,
      ),
    ),
  );

  // Apply runtimes
  const changedRuntimes = await instrumentAsync('applyRuntimes', () =>
    applyRuntimes({
      bundleGraph: internalBundleGraph,
      api,
      config: runner.config,
      options,
      optionsRef,
      pluginOptions: runner.pluginOptions,
      previousDevDeps: runner.previousDevDeps,
      devDepRequests: runner.devDepRequests,
      configs: runner.configs,
    }),
  );

  // Add dev deps for namers
  for (const namer of namers) {
    const devDepRequest = await createDevDependency(
      {
        specifier: namer.name,
        resolveFrom: namer.resolveFrom,
      },
      runner.previousDevDeps,
      options,
    );
    await runDevDepRequest(api, devDepRequest, runner.devDepRequests);
  }

  validateBundles(internalBundleGraph);

  // Pre-compute the hashes for each bundle
  internalBundleGraph.getBundleGraphHash();

  await dumpGraphToGraphViz(
    // @ts-expect-error TS2345
    internalBundleGraph._graph,
    'after_runtimes_native',
    bundleGraphEdgeTypes,
  );

  api.storeResult(
    {
      bundleGraph: internalBundleGraph,
      assetGraphBundlingVersion: assetGraph.getBundlingVersion(),
      changedAssets: new Map(),
      assetRequests: [],
    },
    runner.cacheKey,
  );

  return {
    bundleGraph: internalBundleGraph,
    assetGraphBundlingVersion: assetGraph.getBundlingVersion(),
    changedAssets: changedRuntimes,
    assetRequests,
  };
}

/**
 * Helper class that handles naming bundles and applying runtimes for native bundling.
 * This reuses the same logic as BundlerRunner in BundleGraphRequest.ts.
 */
class NativeBundlerRunner {
  options: AtlaspackOptions;
  optionsRef: SharedReference;
  config!: AtlaspackConfig;
  pluginOptions: PluginOptions;
  api: RunAPI<BundleGraphResult>;
  previousDevDeps: Map<string, string>;
  devDepRequests: Map<string, DevDepRequest | DevDepRequestRef>;
  configs: Map<string, Config>;
  cacheKey: string;

  constructor(
    {input, api, options}: RunInput,
    optionsRef: SharedReference,
  ) {
    this.options = options;
    this.api = api;
    this.optionsRef = optionsRef;
    this.previousDevDeps = new Map();
    this.devDepRequests = new Map();
    this.configs = new Map();
    this.pluginOptions = new PluginOptions(
      optionsProxy(this.options, api.invalidateOnOptionChange),
    );

    const key = hashString(
      `${ATLASPACK_VERSION}:BundleGraph:${
        JSON.stringify(options.entries) ?? ''
      }${options.mode}${options.shouldBuildLazily ? 'lazy' : 'eager'}`,
    );
    this.cacheKey = `BundleGraph/${ATLASPACK_VERSION}/${options.mode}/${key}`;
  }

  async loadConfigs() {
    // Load config using the same pattern as BundleGraphRequest
    const configResult = nullthrows(
      await this.api.runRequest<null, ConfigAndCachePath>(
        createAtlaspackConfigRequest(),
      ),
    );

    this.config = getCachedAtlaspackConfig(configResult, this.options);

    const {devDeps, invalidDevDeps} = await getDevDepRequests(this.api);
    this.previousDevDeps = devDeps;
    invalidateDevDeps(invalidDevDeps, this.options, this.config);

    // Load all configs up front
    const bundler = await this.config.getBundler();
    await this.loadPluginConfig(bundler);

    const namers = await this.config.getNamers();
    for (const namer of namers) {
      await this.loadPluginConfig(namer);
    }

    const runtimes = await this.config.getRuntimes();
    for (const runtime of runtimes) {
      await this.loadPluginConfig(runtime);
    }
  }

  async loadPluginConfig<T extends PluginWithLoadConfig>(
    plugin: LoadedPlugin<T>,
  ) {
    await loadPluginConfigWithDevDeps(
      plugin,
      this.options,
      this.api,
      this.previousDevDeps,
      this.devDepRequests,
      this.configs,
    );
  }
}
