import invariant from 'assert';

import ThrowableDiagnostic from '@atlaspack/diagnostic';
import type {Async} from '@atlaspack/types';
import {ContentGraph} from '@atlaspack/graph';
import {instrument, instrumentAsync, PluginLogger} from '@atlaspack/logger';
import {getFeatureFlag} from '@atlaspack/feature-flags';

import InternalBundleGraph, {bundleGraphEdgeTypes} from '../BundleGraph';
import dumpGraphToGraphViz from '../dumpGraphToGraphViz';
import nullthrows from 'nullthrows';
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
import {requestTypes, StaticRunOpts} from '../RequestTracker';
import type {AtlaspackV3} from '../atlaspack-v3';
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
import {toEnvironmentRef} from '../EnvironmentManager';
import {getEnvironmentHash} from '../Environment';
import type {
  Asset,
  BundleGraphNode,
  BundleNode,
  BundleGroupNode,
  DependencyNode,
  AssetNode,
  Environment,
} from '../types';
import {tracer, PluginTracer} from '@atlaspack/profiler';
import ThrowableDiagnostic2, {errorToDiagnostic} from '@atlaspack/diagnostic';
import type {AtlaspackConfig, LoadedPlugin} from '../AtlaspackConfig';
import type {RunAPI} from '../RequestTracker';
import type {
  Config,
  DevDepRequest,
  AtlaspackOptions,
  DevDepRequestRef,
  Bundle as InternalBundle,
} from '../types';
import type {Namer, Bundle as IBundle} from '@atlaspack/types';
import BundleGraph from '../public/BundleGraph';
import {Bundle, NamedBundle} from '../public/Bundle';

type BundleGraphRequestInput = {
  requestedAssetIds: Set<string>;
  signal?: AbortSignal;
  optionsRef: any;
};

type RunInput = {
  input: BundleGraphRequestInput;
} & StaticRunOpts<BundleGraphResult>;

type BundleGraphRequestRust = {
  id: string;
  readonly type: typeof requestTypes.bundle_graph_request;
  run: (arg1: RunInput) => Async<BundleGraphResult>;
  input: BundleGraphRequestInput;
};

type SerializedBundleGraph = {
  nodes: Array<any>;
  edges: Array<number>;
  publicIdByAssetId: {[k: string]: string};
  assetPublicIds: Array<string>;
  hadPreviousGraph: boolean;
};

export default function createBundleGraphRequestRust(
  input: BundleGraphRequestInput,
): BundleGraphRequestRust {
  return {
    type: requestTypes.bundle_graph_request,
    id: 'BundleGraphRust',
    run: async (runInput) => {
      const {api, options, rustAtlaspack} = runInput;
      invariant(rustAtlaspack, 'BundleGraphRequestRust requires rustAtlaspack');

      let {bundleGraphPromise, commitPromise} =
        await rustAtlaspack.buildBundleGraph();
      let [serializedBundleGraph, bundleGraphError] =
        (await bundleGraphPromise) as [SerializedBundleGraph, Error | null];

      if (bundleGraphError) {
        throw new ThrowableDiagnostic({diagnostic: bundleGraphError});
      }

      // Don’t reuse previous JS result yet; we just rebuild from scratch.
      let {bundleGraph, changedAssets} = instrument(
        'atlaspack_v3_getBundleGraph',
        () => getBundleGraph(serializedBundleGraph),
      );

      const runner = new NativeBundlerRunner(
        {api, options} as any,
        input.optionsRef,
      );
      await runner.loadConfigs();

      // Name all bundles
      const namers = await runner.config.getNamers();
      const bundles = bundleGraph.getBundles({includeInline: true});
      await Promise.all(
        bundles.map((b) =>
          nameBundle(
            namers,
            b,
            bundleGraph,
            options,
            runner.pluginOptions,
            runner.configs,
          ),
        ),
      );

      // Apply runtimes
      const changedRuntimes = await instrumentAsync('applyRuntimes', () =>
        applyRuntimes({
          bundleGraph,
          api,
          config: runner.config,
          options,
          optionsRef: input.optionsRef,
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

      validateBundles(bundleGraph);
      bundleGraph.getBundleGraphHash();

      await dumpGraphToGraphViz(
        // @ts-expect-error Accessing internal graph for debug output
        bundleGraph._graph,
        'after_runtimes_native',
        bundleGraphEdgeTypes,
      );

      let [_commitResult, commitError] = await commitPromise;
      if (commitError) {
        throw new ThrowableDiagnostic({
          diagnostic: {
            message:
              'Error committing bundle graph in Rust: ' + commitError.message,
          },
        });
      }

      return {
        bundleGraph,
        // Not accurate yet — ok for now.
        assetGraphBundlingVersion: 0,
        changedAssets: changedRuntimes,
        assetRequests: [],
        didIncrementallyBundle: false,
      };
    },
    input,
  };
}

function mapSymbols({exported, ...symbol}: any) {
  let jsSymbol: any = {
    local: symbol.local ?? undefined,
    loc: symbol.loc ?? undefined,
    isWeak: symbol.isWeak,
    meta: {
      isEsm: symbol.isEsmExport,
      isStaticBindingSafe: symbol.isStaticBindingSafe,
    },
  };

  if (symbol.exported) {
    jsSymbol.exported = symbol.exported;
  }

  return [exported, jsSymbol];
}

function normalizeEnv(env: Environment): any {
  if (!env) return env;
  env.id = env.id || getEnvironmentHash(env);
  return toEnvironmentRef(env);
}

export function getBundleGraph(serializedGraph: SerializedBundleGraph): {
  bundleGraph: InternalBundleGraph;
  changedAssets: Map<string, Asset>;
} {
  // Build a fresh internal bundle graph.
  const publicIdByAssetId = new Map(
    Object.entries(serializedGraph.publicIdByAssetId ?? {}),
  );
  const assetPublicIds = new Set(serializedGraph.assetPublicIds ?? []);

  // BundleGraph constructor expects a ContentGraph under `_graph`.
  // We reuse the internal graph class by creating an empty instance and then adding nodes.
  const graph = new InternalBundleGraph({
    // We intentionally start with an empty graph and add nodes/edges from the Rust payload.
    // `ContentGraph` will allocate as needed.
    graph: new ContentGraph(),
    bundleContentHashes: new Map(),
    publicIdByAssetId,
    assetPublicIds,
    conditions: new Map(),
  });

  // Root must exist at node id 0.
  const rootNodeId = graph._graph.addNodeByContentKey('@@root', {
    id: '@@root',
    type: 'root',
    value: null,
  });
  graph._graph.setRootNodeId(rootNodeId);

  let entry = 0;
  const changedAssets = new Map<string, Asset>();

  const decoder = new TextDecoder();

  // Create nodes in order.
  for (let i = 0; i < serializedGraph.nodes.length; i++) {
    // Nodes come back as buffers (same as AssetGraphRequestRust)
    let node = JSON.parse(decoder.decode(serializedGraph.nodes[i]));

    if (node.type === 'root') {
      continue;
    }

    if (node.type === 'entry') {
      let id = 'entry:' + ++entry;
      graph._graph.addNodeByContentKey(id, {id, type: 'root', value: null});
      continue;
    }

    if (node.type === 'asset') {
      let asset = node.value;
      let id = asset.id;

      asset.committed = true;
      asset.contentKey = id;
      asset.env = {...asset.env};
      asset.env.id = getFeatureFlag('environmentDeduplication')
        ? getEnvironmentHash(asset.env)
        : getEnvironmentHash(asset.env);
      asset.env = normalizeEnv(asset.env);
      asset.mapKey = `map:${asset.id}`;
      asset.dependencies = new Map();
      asset.meta.isV3 = true;
      if (asset.symbols != null) {
        asset.symbols = new Map(asset.symbols.map(mapSymbols));
      }

      changedAssets.set(id, asset);

      const assetNode: AssetNode = {
        id,
        type: 'asset',
        usedSymbols: new Set(),
        usedSymbolsDownDirty: true,
        usedSymbolsUpDirty: true,
        value: asset,
      };
      graph._graph.addNodeByContentKey(id, assetNode);
      continue;
    }

    if (node.type === 'dependency') {
      let {dependency, id} = node.value;
      dependency.id = id;
      dependency.env = {...dependency.env};
      dependency.env.id = getEnvironmentHash(dependency.env);
      dependency.env = normalizeEnv(dependency.env);
      if (dependency.symbols != null) {
        dependency.symbols = new Map(dependency.symbols?.map(mapSymbols));
      }

      let usedSymbolsDown = new Set();
      let usedSymbolsUp = new Map();
      if (dependency.isEntry && dependency.isLibrary) {
        usedSymbolsDown.add('*');
        usedSymbolsUp.set('*', undefined);
      }

      const depNode: DependencyNode = {
        id,
        type: 'dependency',
        deferred: false,
        excluded: false,
        hasDeferred: node.has_deferred,
        // @ts-expect-error Flow types expect a more specific symbol set type
        usedSymbolsDown,
        usedSymbolsDownDirty: true,
        usedSymbolsUp,
        usedSymbolsUpDirtyDown: true,
        usedSymbolsUpDirtyUp: true,
        value: dependency,
      };
      graph._graph.addNodeByContentKey(id, depNode);
      continue;
    }

    if (node.type === 'bundle') {
      node.value.env = normalizeEnv(node.value.env);
      node.value.target.env = normalizeEnv(node.value.target.env);
      graph._graph.addNodeByContentKey(node.id, node as BundleNode);
      continue;
    }

    if (node.type === 'bundle_group' || node.type === 'bundleGroup') {
      // Rust serializer may emit bundleGroup nodes either as `{id,type,value:{...}}`
      // or as `{type:"bundleGroup", id, target, entryAssetId}`.
      if (node.value == null) {
        node.value = {
          target: node.target,
          entryAssetId: node.entryAssetId ?? node.entry_asset_id,
        };
      }

      // Normalize entry asset id field naming
      if (
        node.value.entryAssetId == null &&
        node.value.entry_asset_id != null
      ) {
        node.value.entryAssetId = node.value.entry_asset_id;
      }

      node.value.target.env = normalizeEnv(node.value.target.env);
      // Normalise to the expected snake_case type
      node.type = 'bundle_group';
      graph._graph.addNodeByContentKey(node.id, node as BundleGroupNode);
      continue;
    }
  }

  // Apply edges
  for (let i = 0; i < serializedGraph.edges.length; i += 3) {
    const from = serializedGraph.edges[i];
    const to = serializedGraph.edges[i + 1];
    const type = serializedGraph.edges[i + 2];

    const fromNode = graph._graph.getNode(from);
    const toNode = graph._graph.getNode(to);

    if (fromNode?.type === 'asset' && toNode?.type === 'dependency') {
      fromNode.value.dependencies.set(toNode.value.id, toNode.value);
    }

    // If we are adding a references edge, remove existing null edge.
    if (
      type === bundleGraphEdgeTypes.references &&
      graph._graph.hasEdge(from, to, bundleGraphEdgeTypes.null)
    ) {
      graph._graph.removeEdge(from, to, bundleGraphEdgeTypes.null);
    }

    graph._graph.addEdge(from, to, type as any);
  }

  return {bundleGraph: graph, changedAssets};
}

class NativeBundlerRunner {
  options: AtlaspackOptions;
  optionsRef: any;
  config!: AtlaspackConfig;
  pluginOptions: PluginOptions;
  api: RunAPI<BundleGraphResult>;
  previousDevDeps: Map<string, string>;
  devDepRequests: Map<string, DevDepRequest | DevDepRequestRef>;
  configs: Map<string, Config>;
  cacheKey: string;

  constructor({api, options}: any, optionsRef: any) {
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
    const configResult = nullthrows(
      await this.api.runRequest<null, ConfigAndCachePath>(
        createAtlaspackConfigRequest(),
      ),
    );

    this.config = getCachedAtlaspackConfig(configResult, this.options);

    const {devDeps, invalidDevDeps} = await getDevDepRequests(this.api);
    this.previousDevDeps = devDeps;
    invalidateDevDeps(invalidDevDeps, this.options, this.config);

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
