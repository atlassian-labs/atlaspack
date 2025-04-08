// @flow strict-local

import invariant from 'assert';

import ThrowableDiagnostic from '@atlaspack/diagnostic';
import type {Async} from '@atlaspack/types';
import {instrument, instrumentAsync} from '@atlaspack/logger';

import AssetGraph, {nodeFromAssetGroup} from '../AssetGraph';
import type {AtlaspackV3} from '../atlaspack-v3';
import {requestTypes, type StaticRunOpts} from '../RequestTracker';
import {propagateSymbols} from '../SymbolPropagation';
import type {Environment, Asset} from '../types';

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

      let serializedAssetGraph = await instrumentAsync(
        'rustAtlaspack.buildAssetGraph',
        () => rustAtlaspack.buildAssetGraph(),
      );

      instrument('atlaspack_v3_parse_graph_nodes', () => {
        serializedAssetGraph.nodes = serializedAssetGraph.nodes.map((node) =>
          JSON.parse(node),
        );
        serializedAssetGraph.updates = serializedAssetGraph.updates.map(
          (node) => JSON.parse(node),
        );
      });

      let prevResult =
        await input.api.getPreviousResult<AssetGraphRequestResult>();

      let {assetGraph, changedAssets} = instrument(
        'atlaspack_v3_getAssetGraph',
        () => getAssetGraph(serializedAssetGraph, prevResult?.assetGraph),
      );

      let changedAssetsPropagation = new Set(changedAssets.keys());

      console.log(changedAssetsPropagation.size, 'changed assets');

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

      let result = {
        assetGraph,
        assetRequests: [],
        assetGroupsWithRemovedParents: new Set(),
        changedAssets,
        changedAssetsPropagation,
        previousSymbolPropagationErrors: undefined,
      };

      await input.api.storeResult(result);
      input.api.invalidateOnBuild();

      return result;
    },
    input,
  });
}

export function getAssetGraph(
  // $FlowFixMe
  serializedGraph: any,
  prevAssetGraph: AssetGraph | void,
): {|
  assetGraph: AssetGraph,
  changedAssets: Map<string, Asset>,
|} {
  let graph: AssetGraph;

  instrument('Initialise asset graph', () => {
    if (prevAssetGraph && serializedGraph.updates.length > 0) {
      graph = new AssetGraph({
        _contentKeyToNodeId: prevAssetGraph._contentKeyToNodeId,
        _nodeIdToContentKey: prevAssetGraph._nodeIdToContentKey,
        nodes: prevAssetGraph.nodes,
        initialCapacity: serializedGraph.edges.length,
      });
    } else {
      graph = new AssetGraph({
        _contentKeyToNodeId: new Map(),
        _nodeIdToContentKey: new Map(),
        initialCapacity: serializedGraph.edges.length,
      });
      let index = graph.addNodeByContentKey('@@root', {
        id: '@@root',
        type: 'root',
        value: null,
      });

      graph.setRootNodeId(index);
    }

    graph.safeToIncrementallyBundle = true;
  });

  invariant(graph, 'Graph not initialized');

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
      envId = envs.size.toString();
      envs.set(envKey, envId);
    }

    return envId;
  };

  console.log({
    newNodes: serializedGraph.nodes.length,
    updatedNodes: serializedGraph.updates.length,
  });

  instrument('Add/Update nodes', () => {
    for (let node of [...serializedGraph.nodes, ...serializedGraph.updates]) {
      if (node.type === 'entry') {
        let id = 'entry:' + ++entry;

        graph.addNodeByContentKey(id, {
          id: id,
          type: 'root',
          value: null,
        });
      } else if (node.type === 'asset') {
        let asset = node.value;
        let id = asset.id;

        asset.committed = true;
        asset.contentKey = id;
        asset.env.id = getEnvId(asset.env);
        asset.mapKey = `map:${asset.id}`;

        // This is populated later when we map the edges between assets and dependencies
        asset.dependencies = new Map();

        // We need to add this property for source map handling, as some assets like runtimes
        // are processed after the Rust transformation and take the v2 code path
        asset.meta.isV3 = true;

        if (asset.symbols != null) {
          asset.symbols = new Map(asset.symbols.map(mapSymbols));
        }

        changedAssets.set(id, asset);

        graph.addNode({
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

        dependency.id = id;
        dependency.env.id = getEnvId(dependency.env);

        if (dependency.symbols != null) {
          dependency.symbols = new Map(dependency.symbols?.map(mapSymbols));
        }

        let usedSymbolsDown = new Set();
        let usedSymbolsUp = new Map();
        if (dependency.isEntry && dependency.isLibrary) {
          usedSymbolsDown.add('*');
          usedSymbolsUp.set('*', undefined);
        }

        graph.addNode({
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
  });

  instrument('Create edges', () => {
    for (let i = 0; i < serializedGraph.edges.length; i += 2) {
      let from = serializedGraph.edges[i];
      let to = serializedGraph.edges[i + 1];
      let fromNode = graph.getNode(from);
      let toNode = graph.getNode(to);

      if (fromNode?.type === 'dependency') {
        invariant(toNode?.type === 'asset');

        // For backwards compatibility, create asset group node if needed.
        // let assetGroupNode = nodeFromAssetGroup({
        //   filePath: toNode.value.filePath,
        //   env: fromNode.value.env,
        //   pipeline: toNode.value.pipeline,
        //   sideEffects: Boolean(toNode.value.sideEffects),
        // });
        //
        // let index = graph.addNodeByContentKeyIfNeeded(
        //   assetGroupNode.id,
        //   assetGroupNode,
        // );

        // graph.addEdge(from, index);
        // graph.addEdge(index, to);
        graph.addEdge(from, to);
      } else if (fromNode?.type === 'asset' && toNode?.type === 'dependency') {
        fromNode.value.dependencies.set(toNode.value.id, toNode.value);
        graph.addEdge(from, to);
      } else {
        graph.addEdge(from, to);
      }
    }
  });

  return {
    assetGraph: graph,
    changedAssets,
  };
}
