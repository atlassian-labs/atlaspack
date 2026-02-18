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
import type {ConfigAndCachePath} from './AtlaspackConfigRequest';

import fs from 'fs';
import invariant from 'assert';
import assert from 'assert';
import nullthrows from 'nullthrows';
import {instrumentAsync, PluginLogger} from '@atlaspack/logger';
import {getFeatureFlag} from '@atlaspack/feature-flags';
import ThrowableDiagnostic, {errorToDiagnostic} from '@atlaspack/diagnostic';
import AssetGraph from '../AssetGraph';
import InternalBundleGraph, {bundleGraphEdgeTypes} from '../BundleGraph';
import MutableBundleGraph from '../public/MutableBundleGraph';
import {report} from '../ReporterRunner';
import dumpGraphToGraphViz from '../dumpGraphToGraphViz';
import {hashString} from '@atlaspack/rust';
import PluginOptions from '../public/PluginOptions';
import applyRuntimes from '../applyRuntimes';
import {ATLASPACK_VERSION, OPTION_CHANGE} from '../constants';
import {assertSignalNotAborted, optionsProxy} from '../utils';
import createAtlaspackConfigRequest, {
  getCachedAtlaspackConfig,
} from './AtlaspackConfigRequest';
import {
  createDevDependency,
  getDevDepRequests,
  invalidateDevDeps,
  runDevDepRequest,
} from './DevDepRequest';
import {
  loadPluginConfig,
  runConfigRequest,
  PluginWithLoadConfig,
} from './ConfigRequest';
import {fromProjectPathRelative} from '../projectPath';
import {
  validateBundles,
  nameBundle,
  loadPluginConfigWithDevDeps,
  runDevDepRequest as runDevDepRequestShared,
} from './BundleGraphRequestUtils';
import createAssetGraphRequestJS from './AssetGraphRequest';
import {createAssetGraphRequestRust} from './AssetGraphRequestRust';
import {tracer, PluginTracer} from '@atlaspack/profiler';
import {requestTypes} from '../RequestTracker';
import {
  assetGraphToDot,
  getDebugAssetGraphDotPath,
  getDebugAssetGraphDotOptions,
} from './asset-graph-dot';
import {Priority} from '../types';

type BundleGraphRequestInput = {
  requestedAssetIds: Set<string>;
  signal?: AbortSignal;
  optionsRef: SharedReference;
};

type RunInput = {
  input: BundleGraphRequestInput;
} & StaticRunOpts<BundleGraphResult>;

// TODO: Rename to BundleGraphRequestResult
export type BundleGraphResult = {
  bundleGraph: InternalBundleGraph;
  assetGraphBundlingVersion: number;
  changedAssets: Map<string, Asset>;
  assetRequests: Array<AssetGroup>;
};

type BundleGraphRequest = {
  id: string;
  readonly type: typeof requestTypes.bundle_graph_request;
  run: (arg1: RunInput) => Async<BundleGraphResult>;
  input: BundleGraphRequestInput;
};

function applySideEffectsForLoadableImports(assetGraph: AssetGraph) {
  // Avoid revisiting nodes
  let updatedAssetIds = new Set();

  assetGraph.traverse((nodeId) => {
    let node = nullthrows(assetGraph.getNode(nodeId));

    if (
      node.type !== 'dependency' ||
      node.value.specifier.indexOf('@confluence/loadable') === -1
    ) {
      return;
    }

    assetGraph.traverseAncestors(nodeId, (ancestorNodeId, _, actions) => {
      if (updatedAssetIds.has(ancestorNodeId)) {
        actions.skipChildren();
        return;
      }

      let ancestorNode = nullthrows(assetGraph.getNode(ancestorNodeId));

      // Async boundaries will catch the side effects
      if (
        ancestorNode.type === 'dependency' &&
        ancestorNode.value.priority !== Priority.sync
      ) {
        actions.skipChildren();
      }

      // inline-requires optimizer is only checking assets
      if (ancestorNode.type !== 'asset') {
        return;
      }

      updatedAssetIds.add(ancestorNodeId);
      ancestorNode.value.sideEffects = true;
    });
  }, assetGraph.rootNodeId);
}

export default function createBundleGraphRequest(
  input: BundleGraphRequestInput,
): BundleGraphRequest {
  return {
    type: requestTypes.bundle_graph_request,
    id: 'BundleGraph',
    run: async (input) => {
      let {options, api, invalidateReason} = input;
      let {optionsRef, requestedAssetIds, signal} = input.input;
      let measurement = tracer.createMeasurement('building');

      let createAssetGraphRequest =
        getFeatureFlag('atlaspackV3') && input.rustAtlaspack
          ? createAssetGraphRequestRust(input.rustAtlaspack)
          : createAssetGraphRequestJS;

      let request = createAssetGraphRequest({
        name: 'Main',
        entries: options.entries,
        optionsRef,
        shouldBuildLazily: options.shouldBuildLazily,
        lazyIncludes: options.lazyIncludes,
        lazyExcludes: options.lazyExcludes,
        requestedAssetIds,
      });

      let {assetGraph, changedAssets, assetRequests} = await instrumentAsync(
        'asset-graph-request',
        () => {
          return api.runRequest(request, {
            force:
              Boolean(input.rustAtlaspack) ||
              (options.shouldBuildLazily && requestedAssetIds.size > 0),
          });
        },
      );

      if (input.options.featureFlags?.loadableSideEffects) {
        applySideEffectsForLoadableImports(assetGraph);
      }

      let debugAssetGraphDotPath = getDebugAssetGraphDotPath();
      if (debugAssetGraphDotPath !== null) {
        await fs.promises.writeFile(
          debugAssetGraphDotPath,
          assetGraphToDot(assetGraph, getDebugAssetGraphDotOptions()),
          'utf8',
        );
      }

      // if (input.rustAtlaspack && process.env.NATIVE_COMPARE === 'true') {
      //   let {assetGraph: jsAssetGraph} = await api.runRequest(
      //     createAssetGraphRequestJS({
      //       name: 'Main',
      //       entries: options.entries,
      //       optionsRef,
      //       shouldBuildLazily: options.shouldBuildLazily,
      //       lazyIncludes: options.lazyIncludes,
      //       lazyExcludes: options.lazyExcludes,
      //       requestedAssetIds,
      //     }),
      //     {
      //       force: true,
      //     },
      //   );
      //   require('./asset-graph-diff.ts')(
      //     jsAssetGraph,
      //     assetGraph,
      //     options.projectRoot,
      //   );
      // }

      measurement && measurement.end();
      assertSignalNotAborted(signal);

      // If any subrequests are invalid (e.g. dev dep requests or config requests),
      // bail on incremental bundling. We also need to invalidate for option changes,
      // which are hoisted to direct invalidations on the bundle graph request.
      let subRequestsInvalid =
        Boolean(invalidateReason & OPTION_CHANGE) ||
        input.api
          .getSubRequests()
          .some((req) => !input.api.canSkipSubrequest(req.id));

      if (subRequestsInvalid) {
        assetGraph.safeToIncrementallyBundle = false;
        assetGraph.setNeedsBundling();
      }

      let configResult = nullthrows(
        await input.api.runRequest<null, ConfigAndCachePath>(
          createAtlaspackConfigRequest(),
        ),
      );

      assertSignalNotAborted(signal);

      let atlaspackConfig = getCachedAtlaspackConfig(
        configResult,
        input.options,
      );
      let {devDeps, invalidDevDeps} = await getDevDepRequests(input.api);
      invalidateDevDeps(invalidDevDeps, input.options, atlaspackConfig);

      let bundlingMeasurement = tracer.createMeasurement('bundling');
      let builder = new BundlerRunner(input, atlaspackConfig, devDeps);
      let res: BundleGraphResult = await builder.bundle({
        graph: assetGraph,
        changedAssets: changedAssets,
        assetRequests,
      });
      bundlingMeasurement && bundlingMeasurement.end();
      for (let [id, asset] of changedAssets) {
        res.changedAssets.set(id, asset);
      }

      await dumpGraphToGraphViz(
        // @ts-expect-error TS2345
        res.bundleGraph._graph,
        'BundleGraph',
        bundleGraphEdgeTypes,
      );

      return res;
    },
    input,
  };
}

class BundlerRunner {
  options: AtlaspackOptions;
  optionsRef: SharedReference;
  config: AtlaspackConfig;
  pluginOptions: PluginOptions;
  api: RunAPI<BundleGraphResult>;
  previousDevDeps: Map<string, string>;
  devDepRequests: Map<string, DevDepRequest | DevDepRequestRef>;
  configs: Map<string, Config>;
  cacheKey: string;

  constructor(
    {input, api, options}: RunInput,
    config: AtlaspackConfig,
    previousDevDeps: Map<string, string>,
  ) {
    this.options = options;
    this.api = api;
    this.optionsRef = input.optionsRef;
    this.config = config;
    this.previousDevDeps = previousDevDeps;
    this.devDepRequests = new Map();
    this.configs = new Map();
    this.pluginOptions = new PluginOptions(
      optionsProxy(this.options, api.invalidateOnOptionChange),
    );
    this.cacheKey =
      hashString(
        `${ATLASPACK_VERSION}:BundleGraph:${
          JSON.stringify(options.entries) ?? ''
        }${options.mode}${options.shouldBuildLazily ? 'lazy' : 'eager'}`,
      ) + '-BundleGraph';
  }

  async loadConfigs() {
    // Load all configs up front so we can use them in the cache key
    let bundler = await this.config.getBundler();
    await this.loadConfig(bundler);

    let namers = await this.config.getNamers();
    for (let namer of namers) {
      await this.loadConfig(namer);
    }

    let runtimes = await this.config.getRuntimes();
    for (let runtime of runtimes) {
      await this.loadConfig(runtime);
    }
  }

  async loadConfig<T extends PluginWithLoadConfig>(plugin: LoadedPlugin<T>) {
    await loadPluginConfigWithDevDeps(
      plugin,
      this.options,
      this.api,
      this.previousDevDeps,
      this.devDepRequests,
      this.configs,
    );
  }

  async runDevDepRequest(devDepRequest: DevDepRequest | DevDepRequestRef) {
    await runDevDepRequestShared(this.api, devDepRequest, this.devDepRequests);
  }

  async bundle({
    graph,
    changedAssets,
    assetRequests,
  }: {
    graph: AssetGraph;
    changedAssets: Map<string, Asset>;
    assetRequests: Array<AssetGroup>;
  }): Promise<BundleGraphResult> {
    report({
      type: 'buildProgress',
      phase: 'bundling',
    });

    await this.loadConfigs();

    let plugin = await this.config.getBundler();
    let {plugin: bundler, name, resolveFrom} = plugin;

    // if a previous asset graph hash is passed in, check if the bundle graph is also available
    const previousBundleGraphResult: BundleGraphResult | null | undefined =
      await this.api.getPreviousResult();
    const canIncrementallyBundle =
      previousBundleGraphResult?.assetGraphBundlingVersion != null &&
      graph.canIncrementallyBundle(
        previousBundleGraphResult.assetGraphBundlingVersion,
      );

    if (graph.safeToIncrementallyBundle && previousBundleGraphResult == null) {
      graph.safeToIncrementallyBundle = false;
      graph.setNeedsBundling();
    }

    let internalBundleGraph;

    let logger = new PluginLogger({origin: name});
    let tracer = new PluginTracer({
      origin: name,
      category: 'bundle',
    });
    try {
      if (canIncrementallyBundle && previousBundleGraphResult) {
        internalBundleGraph = previousBundleGraphResult.bundleGraph;
        for (let changedAssetId of changedAssets.keys()) {
          // Copy over the whole node to also have correct symbol data
          let changedAssetNode = nullthrows(
            graph.getNodeByContentKey(changedAssetId),
          );
          invariant(changedAssetNode.type === 'asset');
          internalBundleGraph.updateAsset(changedAssetNode);
        }
      } else {
        internalBundleGraph = InternalBundleGraph.fromAssetGraph(
          graph,
          this.options.mode === 'production',
        );
        invariant(internalBundleGraph != null); // ensures the graph was created

        await dumpGraphToGraphViz(
          // @ts-expect-error TS2345
          internalBundleGraph._graph,
          'before_bundle',
          bundleGraphEdgeTypes,
        );
        let mutableBundleGraph = new MutableBundleGraph(
          internalBundleGraph,
          this.options,
        );

        let measurement;
        let measurementFilename;
        if (tracer.enabled) {
          measurementFilename = graph
            .getEntryAssets()
            .map((asset) => fromProjectPathRelative(asset.filePath))
            .join(', ');
          measurement = tracer.createMeasurement(
            plugin.name,
            'bundling:bundle',
            measurementFilename,
          );
        }

        // this the normal bundle workflow (bundle, optimizing, run-times, naming)
        await bundler.bundle({
          bundleGraph: mutableBundleGraph,
          config: this.configs.get(plugin.name)?.result,
          options: this.pluginOptions,
          logger,
          tracer,
        });

        measurement && measurement.end();

        if (this.pluginOptions.mode === 'production') {
          let optimizeMeasurement;
          try {
            if (tracer.enabled) {
              optimizeMeasurement = tracer.createMeasurement(
                plugin.name,
                'bundling:optimize',
                nullthrows(measurementFilename),
              );
            }
            await bundler.optimize({
              bundleGraph: mutableBundleGraph,
              config: this.configs.get(plugin.name)?.result,
              options: this.pluginOptions,
              logger,
            });
          } catch (e: any) {
            throw new ThrowableDiagnostic({
              diagnostic: errorToDiagnostic(e, {
                origin: plugin.name,
              }),
            });
          } finally {
            optimizeMeasurement && optimizeMeasurement.end();
            await dumpGraphToGraphViz(
              // @ts-expect-error TS2345
              internalBundleGraph._graph,
              'after_optimize',
            );
          }
        }

        // Add dev dependency for the bundler. This must be done AFTER running it due to
        // the potential for lazy require() that aren't executed until the request runs.
        let devDepRequest = await createDevDependency(
          {
            specifier: name,
            resolveFrom,
          },
          this.previousDevDeps,
          this.options,
        );
        await this.runDevDepRequest(devDepRequest);
      }
    } catch (e: any) {
      if (internalBundleGraph != null) {
        this.api.storeResult(
          {
            bundleGraph: internalBundleGraph,
            assetGraphBundlingVersion: graph.getBundlingVersion(),
            changedAssets: new Map(),
            assetRequests: [],
          },
          this.cacheKey,
        );
      }

      throw new ThrowableDiagnostic({
        diagnostic: errorToDiagnostic(e, {
          origin: name,
        }),
      });
    } finally {
      if (internalBundleGraph != null) {
        await dumpGraphToGraphViz(
          // @ts-expect-error TS2345
          internalBundleGraph._graph,
          'after_bundle',
          bundleGraphEdgeTypes,
        );
      }
    }

    let changedRuntimes = new Map();
    if (!previousBundleGraphResult || !canIncrementallyBundle) {
      let namers = await this.config.getNamers();
      // inline bundles must still be named so the PackagerRunner
      // can match them to the correct packager/optimizer plugins.
      let bundles = internalBundleGraph.getBundles({includeInline: true});
      await Promise.all(
        bundles.map((bundle) =>
          nameBundle(
            namers,
            bundle,
            internalBundleGraph,
            this.options,
            this.pluginOptions,
            this.configs,
          ),
        ),
      );

      changedRuntimes = await instrumentAsync('applyRuntimes', () =>
        applyRuntimes({
          bundleGraph: internalBundleGraph,
          api: this.api,
          config: this.config,
          options: this.options,
          optionsRef: this.optionsRef,
          pluginOptions: this.pluginOptions,
          previousDevDeps: this.previousDevDeps,
          devDepRequests: this.devDepRequests,
          configs: this.configs,
        }),
      );

      // Add dev deps for namers, AFTER running them to account for lazy require().
      for (let namer of namers) {
        let devDepRequest = await createDevDependency(
          {
            specifier: namer.name,
            resolveFrom: namer.resolveFrom,
          },
          this.previousDevDeps,
          this.options,
        );
        await this.runDevDepRequest(devDepRequest);
      }

      validateBundles(internalBundleGraph);

      // Pre-compute the hashes for each bundle so they are only computed once and shared between workers.
      internalBundleGraph.getBundleGraphHash();
    }

    await dumpGraphToGraphViz(
      // @ts-expect-error TS2345
      internalBundleGraph._graph,
      'after_runtimes',
      bundleGraphEdgeTypes,
    );

    this.api.storeResult(
      {
        bundleGraph: internalBundleGraph,
        assetGraphBundlingVersion: graph.getBundlingVersion(),
        changedAssets: new Map(),
        assetRequests: [],
      },
      this.cacheKey,
    );

    return {
      bundleGraph: internalBundleGraph,
      assetGraphBundlingVersion: graph.getBundlingVersion(),
      changedAssets: changedRuntimes,
      assetRequests,
    };
  }
}
