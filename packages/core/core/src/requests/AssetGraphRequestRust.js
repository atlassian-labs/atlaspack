// @flow strict-local

import invariant from 'assert';

import ThrowableDiagnostic from '@atlaspack/diagnostic';
import {hashString} from '@atlaspack/rust';
import type {Async} from '@atlaspack/types';

import AssetGraph, {nodeFromAssetGroup} from '../AssetGraph';
import type {AtlaspackV3} from '../atlaspack-v3';
import {ATLASPACK_VERSION} from '../constants';
import {toProjectPath} from '../projectPath';
import {requestTypes, type StaticRunOpts} from '../RequestTracker';
import {propagateSymbols} from '../SymbolPropagation';
import type {Environment} from '../types';

import type {
  AssetGraphRequestInput,
  AssetGraphRequestResult,
} from './AssetGraphRequest';

type RunInput = {|
  input: AssetGraphRequestInput,
  ...StaticRunOpts<AssetGraphRequestResult>,
|};

type AssetGraphRequest = {|
  id: string,
  +type: typeof requestTypes.asset_graph_request,
  run: (RunInput) => Async<AssetGraphRequestResult>,
  input: AssetGraphRequestInput,
|};

export function createAssetGraphRequestRust(
  rustAtlaspack: AtlaspackV3,
): (input: AssetGraphRequestInput) => AssetGraphRequest {
  return (input) => ({
    type: requestTypes.asset_graph_request,
    id: input.name,
    run: async (input) => {
      let options = input.options;
      let serializedAssetGraph;
      try {
        serializedAssetGraph = await rustAtlaspack.buildAssetGraph();
      } catch (err) {
        throw new ThrowableDiagnostic({
          diagnostic: err,
        });
      }

      let {assetGraph, cachedAssets, changedAssets} = getAssetGraph(
        serializedAssetGraph,
        options,
      );

      // TODO: Make it a bulk transaction
      await Promise.all(
        Array.from(cachedAssets.entries(), ([id, code]) =>
          options.cache.setBlob(id, Buffer.from(code)),
        ),
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

function getAssetGraph(serializedGraph, options) {
  let graph = new AssetGraph({
    _contentKeyToNodeId: new Map(),
    _nodeIdToContentKey: new Map(),
  });

  graph.safeToIncrementallyBundle = false;

  function mapSymbols({exported, ...symbol}) {
    let jsSymbol = {
      local: symbol.local ?? undefined,
      loc: symbol.loc ?? undefined,
      meta: undefined,
      isWeak: symbol.isWeak,
    };

    if (symbol.exported) {
      // $FlowFixMe
      jsSymbol.exported = symbol.exported;
    }

    if (symbol.isEsmExport) {
      // $FlowFixMe
      jsSymbol.meta = {
        isEsm: true,
      };
    }
    return [exported, jsSymbol];
  }

  // See crates/atlaspack_core/src/types/environment.rs
  let cachedAssets = new Map();
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
      envId = envs.size;
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

      asset.meta.id = id;

      asset = {
        ...asset,
        uniqueKey: asset.uniqueKey ?? undefined,
        pipeline: asset.pipeline ?? undefined,
        range: asset.range ?? undefined,
        resolveFrom: asset.resolveFrom ?? undefined,
        target: asset.target ?? undefined,
        plugin: asset.plugin ?? undefined,
        query: asset.query ?? undefined,
        configPath: asset.configPath ?? undefined,
        configKeyPath: asset.configKeyPath ?? undefined,
        isLargeBlob: asset.isLargeBlob ?? false,
        isSource: asset.isSource ?? false,
        sourcePath: asset.sourcePath ?? undefined,
        env: {
          ...asset.env,
          loc: asset.env.loc ?? undefined,
          id: getEnvId(asset.env),
          sourceType: asset.env.sourceType,
        },
        bundleBehavior:
          asset.bundleBehavior === 255 ? null : asset.bundleBehavior,
        committed: true,
        contentKey: id,
        filePath: toProjectPath(options.projectRoot, asset.filePath),
        symbols:
          asset.symbols != null ? new Map(asset.symbols.map(mapSymbols)) : null,
      };

      if (asset.map) {
        let mapKey = hashString(`${ATLASPACK_VERSION}:map:${asset.id}`);

        asset.mapKey = mapKey;
        options.cache.setBlob(mapKey, asset.map);
        delete asset.map;
      }

      cachedAssets.set(id, asset.code);
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
      let dependency = node.value.dependency;

      dependency = {
        ...dependency,
        id,
        env: {
          ...dependency.env,
          id: getEnvId(dependency.env),
          sourceType: dependency.env.sourceType,
          loc: dependency.env.loc ?? undefined,
        },
        pipeline: dependency.pipeline ?? undefined,
        range: dependency.range ?? undefined,
        resolveFrom: dependency.resolveFrom ?? undefined,
        target: dependency.target ?? undefined,
        bundleBehavior:
          dependency.bundleBehavior === 255 ? null : dependency.bundleBehavior,
        contentKey: id,
        loc: dependency.loc
          ? {
              ...dependency.loc,
              filePath: toProjectPath(
                options.projectRoot,
                dependency.loc.filePath,
              ),
            }
          : undefined,
        sourcePath: dependency.sourcePath
          ? toProjectPath(options.projectRoot, dependency.sourcePath)
          : undefined,
        symbols:
          // Dependency.symbols are always set to an empty map when scope hoisting
          // is enabled. Some tests will fail if this is not the case. We should
          // make this consistant when we re-visit packaging.
          dependency.symbols != null || dependency.env.shouldScopeHoist
            ? new Map(dependency.symbols?.map(mapSymbols))
            : undefined,
      };
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
    if (fromNode?.type === 'dependency') {
      let toNode = graph.getNode(to);
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
    } else {
      graph.addEdge(from, to);
    }
  }

  return {
    assetGraph: graph,
    cachedAssets,
    changedAssets,
  };
}
