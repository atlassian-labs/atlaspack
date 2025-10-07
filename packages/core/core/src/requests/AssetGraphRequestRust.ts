import invariant from 'assert';

import ThrowableDiagnostic from '@atlaspack/diagnostic';
import type {Async} from '@atlaspack/types';
import {instrument} from '@atlaspack/logger';
import {getFeatureFlag} from '@atlaspack/feature-flags';

import AssetGraph, {nodeFromAssetGroup} from '../AssetGraph';
import type {AtlaspackV3} from '../atlaspack-v3';
import {requestTypes, StaticRunOpts} from '../RequestTracker';
import {propagateSymbols} from '../SymbolPropagation';
import type {Environment, Asset} from '../types';

import type {
  AssetGraphRequestInput,
  AssetGraphRequestResult,
} from './AssetGraphRequest';
import {toEnvironmentRef} from '../EnvironmentManager';
import {getEnvironmentHash} from '../Environment';

type RunInput = {
  input: AssetGraphRequestInput;
} & StaticRunOpts<AssetGraphRequestResult>;

type AssetGraphRequest = {
  id: string;
  readonly type: typeof requestTypes.asset_graph_request;
  run: (arg1: RunInput) => Async<AssetGraphRequestResult>;
  input: AssetGraphRequestInput;
};

export function createAssetGraphRequestRust(
  rustAtlaspack: AtlaspackV3,
): (input: AssetGraphRequestInput) => AssetGraphRequest {
  return (input: AssetGraphRequestInput) => ({
    type: requestTypes.asset_graph_request,
    id: input.name,
    run: async (input) => {
      let options = input.options;
      let serializedAssetGraph = await rustAtlaspack.buildAssetGraph();

      // @ts-expect-error TS7006
      serializedAssetGraph.nodes = serializedAssetGraph.nodes.map((node) =>
        JSON.parse(node),
      );

      let {assetGraph, changedAssets} = instrument(
        'atlaspack_v3_getAssetGraph',
        () => getAssetGraph(serializedAssetGraph),
      );

      let changedAssetsPropagation = new Set(changedAssets.keys());
      let errors = propagateSymbols({
        options,
        assetGraph,
        changedAssetsPropagation,
        assetGroupsWithRemovedParents: new Set(),
        previousErrors: new Map(), //this.previousSymbolPropagationErrors,
      });

      if (errors.size > 0) {
        // Just throw the first error. Since errors can bubble (e.g. reexporting a reexported symbol also fails),
        // determining which failing export is the root cause is nontrivial (because of circular dependencies).
        throw new ThrowableDiagnostic({
          diagnostic: [...errors.values()][0],
        });
      }

      return {
        assetGraph,
        assetRequests: [],
        assetGroupsWithRemovedParents: new Set(),
        changedAssets,
        changedAssetsPropagation,
        previousSymbolPropagationErrors: undefined,
      };
    },
    input,
  });
}

export function getAssetGraph(serializedGraph: any): {
  assetGraph: AssetGraph;
  changedAssets: Map<string, Asset>;
} {
  let graph = new AssetGraph({
    _contentKeyToNodeId: new Map(),
    _nodeIdToContentKey: new Map(),
    initialCapacity: serializedGraph.edges.length,
  });

  graph.safeToIncrementallyBundle = false;

  // @ts-expect-error TS7031
  function mapSymbols({exported, ...symbol}) {
    let jsSymbol = {
      local: symbol.local ?? undefined,
      loc: symbol.loc ?? undefined,
      isWeak: symbol.isWeak,
      meta: {
        isEsm: symbol.isEsmExport,
        isStaticBindingSafe: symbol.isStaticBindingSafe,
      },
    };

    if (symbol.exported) {
      // @ts-expect-error TS2339
      jsSymbol.exported = symbol.exported;
    }

    return [exported, jsSymbol];
  }

  // See crates/atlaspack_core/src/types/environment.rs
  let changedAssets = new Map();
  let entry = 0;

  let envs = new Map();
  let getEnvId = (env: Environment) => {
    let envKey = [
      env.context,
      env.engines.atlaspack,
      env.engines.browsers,
      env.engines.electron,
      env.engines.node,
      env.includeNodeModules,
      env.isLibrary,
      env.outputFormat,
      env.shouldScopeHoist,
      env.shouldOptimize,
      env.sourceType,
    ].join(':');

    let envId = envs.get(envKey);
    if (envId == null) {
      envId = `${envs.size}`;
      envs.set(envKey, envId);
    }

    return envId;
  };

  for (let node of serializedGraph.nodes) {
    if (node.type === 'root') {
      let index = graph.addNodeByContentKey('@@root', {
        id: '@@root',
        type: 'root',
        value: null,
      });

      graph.setRootNodeId(index);
    } else if (node.type === 'entry') {
      let id = 'entry:' + ++entry;

      graph.addNodeByContentKey(id, {
        id: id,
        type: 'root',
        value: null,
      });
    } else if (node.type === 'asset') {
      let asset = node.value;
      let id = asset.id;

      asset.meta = {
        conditions: asset.conditions,
        emptyFileStarReexport: asset.emptyFileStarReexport,
        hasCJSExports: asset.hasCjsExports,
        hasDependencies: asset.hasDependencies,
        hasReferences: asset.hasReferences,
        has_node_replacements: asset.hasNodeReplacements,
        id: asset.packagingId,
        inlineType: asset.inlineType,
        interpreter: asset.interpreter,
        isConstantModule: asset.isConstantModule,
        shouldWrap: asset.shouldWrap,
        staticExports: asset.staticExports,
        type: asset.cssDependencyType,
        ...asset.meta,
      };

      asset.committed = true;
      asset.contentKey = id;
      asset.env.id = getFeatureFlag('environmentDeduplication')
        ? // TODO: Rust can do this and avoid copying a significant amount of data over
          getEnvironmentHash(asset.env)
        : getEnvId(asset.env);
      asset.mapKey = `map:${asset.id}`;

      asset.env = toEnvironmentRef(asset.env);

      // This is populated later when we map the edges between assets and dependencies
      asset.dependencies = new Map();

      // We need to add this property for source map handling, as some assets like runtimes
      // are processed after the Rust transformation and take the v2 code path
      asset.meta.isV3 = true;

      if (asset.symbols != null) {
        asset.symbols = new Map(asset.symbols.map(mapSymbols));
      }

      changedAssets.set(id, asset);

      graph.addNodeByContentKey(id, {
        id,
        type: 'asset',
        usedSymbols: new Set(),
        usedSymbolsDownDirty: true,
        usedSymbolsUpDirty: true,
        value: asset,
      });
    } else if (node.type === 'dependency') {
      let id = node.value.id;
      let {
        isWebworker,
        kind,
        promiseSymbol,
        importAttributes,
        media,
        isCssImport,
        chunkNameMagicComment,
        ...dependency
      } = node.value.dependency;

      // Re-map top level meta fields back into a meta object and remove them
      // from the top level of the dependency.
      dependency.meta = {
        chunkNameMagicComment,
        importAttributes,
        isCSSImport: isCssImport,
        kind,
        media,
        placeholder: dependency.placeholder,
        promiseSymbol,
        shouldWrap: dependency.shouldWrap,
        webworker: isWebworker,
        ...dependency.meta,
      };

      dependency.id = id;
      dependency.env.id = getFeatureFlag('environmentDeduplication')
        ? // TODO: Rust can do this and avoid copying a significant amount of data over
          getEnvironmentHash(dependency.env)
        : getEnvId(dependency.env);
      dependency.env = toEnvironmentRef(dependency.env);

      if (dependency.symbols != null) {
        dependency.symbols = new Map(dependency.symbols?.map(mapSymbols));
      }

      let usedSymbolsDown = new Set();
      let usedSymbolsUp = new Map();
      if (dependency.isEntry && dependency.isLibrary) {
        usedSymbolsDown.add('*');
        usedSymbolsUp.set('*', undefined);
      }

      graph.addNodeByContentKey(id, {
        id,
        type: 'dependency',
        deferred: false,
        excluded: false,
        hasDeferred: node.has_deferred,
        // @ts-expect-error TS2322
        usedSymbolsDown,
        usedSymbolsDownDirty: true,
        usedSymbolsUp,
        usedSymbolsUpDirtyDown: true,
        usedSymbolsUpDirtyUp: true,
        value: dependency,
      });
    }
  }

  for (let i = 0; i < serializedGraph.edges.length; i += 2) {
    let from = serializedGraph.edges[i];
    let to = serializedGraph.edges[i + 1];
    let fromNode = graph.getNode(from);
    let toNode = graph.getNode(to);

    if (fromNode?.type === 'dependency') {
      invariant(toNode?.type === 'asset');

      // For backwards compatibility, create asset group node if needed.
      let assetGroupNode = nodeFromAssetGroup({
        filePath: toNode.value.filePath,
        env: fromNode.value.env,
        pipeline: toNode.value.pipeline,
        sideEffects: Boolean(toNode.value.sideEffects),
      });

      let index = graph.addNodeByContentKeyIfNeeded(
        assetGroupNode.id,
        assetGroupNode,
      );

      graph.addEdge(from, index);
      graph.addEdge(index, to);
    } else if (fromNode?.type === 'asset' && toNode?.type === 'dependency') {
      fromNode.value.dependencies.set(toNode.value.id, toNode.value);
      graph.addEdge(from, to);
    } else {
      graph.addEdge(from, to);
    }
  }

  return {
    assetGraph: graph,
    changedAssets,
  };
}
