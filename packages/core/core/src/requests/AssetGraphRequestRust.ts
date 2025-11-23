import invariant from 'assert';

import ThrowableDiagnostic from '@atlaspack/diagnostic';
import type {Async} from '@atlaspack/types';
import {instrument} from '@atlaspack/logger';
import {getFeatureFlag} from '@atlaspack/feature-flags';

import AssetGraph from '../AssetGraph';
import type {AtlaspackV3} from '../atlaspack-v3';
import {requestTypes, StaticRunOpts} from '../RequestTracker';
import {propagateSymbols} from '../SymbolPropagation';
import type {
  Environment,
  Asset,
  AssetGraphNode,
  AssetNode,
  DependencyNode,
} from '../types';

import type {
  AssetGraphRequestInput,
  AssetGraphRequestResult,
} from './AssetGraphRequest';
import {toEnvironmentRef} from '../EnvironmentManager';
import {getEnvironmentHash} from '../Environment';
import dumpGraphToGraphViz from '../dumpGraphToGraphViz';
import assert from 'assert';

type RunInput = {
  input: AssetGraphRequestInput;
} & StaticRunOpts<AssetGraphRequestResult>;

type AssetGraphRequest = {
  id: string;
  readonly type: typeof requestTypes.asset_graph_request;
  run: (arg1: RunInput) => Async<AssetGraphRequestResult>;
  input: AssetGraphRequestInput;
};

type SerializedAssetGraphDelta = {
  nodes: Array<any>;
  edges: Array<string>;
  updates: Array<any>;
  safeToSkipBundling: boolean;
  hadPreviousGraph: boolean;
};

export function createAssetGraphRequestRust(
  rustAtlaspack: AtlaspackV3,
): (input: AssetGraphRequestInput) => AssetGraphRequest {
  return (input: AssetGraphRequestInput) => ({
    type: requestTypes.asset_graph_request,
    id: input.name,
    run: async (input) => {
      let options = input.options;
      let {assetGraphPromise, commitPromise} =
        await rustAtlaspack.buildAssetGraph();

      let [serializedAssetGraph, assetGraphError] =
        (await assetGraphPromise) as [SerializedAssetGraphDelta, Error | null];

      if (assetGraphError) {
        throw new ThrowableDiagnostic({
          diagnostic: {
            message:
              'Error building asset graph in Rust: ' + assetGraphError.message,
          },
        });
      }

      instrument('parse-asset-graph', () => {
        // Newly created nodes
        serializedAssetGraph.nodes = serializedAssetGraph.nodes.map((node) =>
          JSON.parse(node),
        );

        // Updated existing nodes
        serializedAssetGraph.updates = serializedAssetGraph.updates.map(
          (node) => JSON.parse(node),
        );
      });

      // Don't reuse a previous asset graph result if Rust didn't have one too
      let prevResult = null;
      if (serializedAssetGraph.hadPreviousGraph) {
        prevResult =
          await input.api.getPreviousResult<AssetGraphRequestResult>();
      }

      let {assetGraph, changedAssets} = instrument(
        'atlaspack_v3_getAssetGraph',
        () => getAssetGraph(serializedAssetGraph, prevResult?.assetGraph),
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

      await dumpGraphToGraphViz(assetGraph, 'AssetGraphV3');

      let result = {
        assetGraph,
        assetRequests: [],
        assetGroupsWithRemovedParents: new Set<number>(),
        changedAssets,
        changedAssetsPropagation,
        previousSymbolPropagationErrors: undefined,
      };

      let [_commitResult, commitError] = await commitPromise;

      if (commitError) {
        throw new ThrowableDiagnostic({
          diagnostic: {
            message:
              'Error committing asset graph in Rust: ' + commitError.message,
          },
        });
      }

      await input.api.storeResult(result);
      input.api.invalidateOnBuild();

      return result;
    },
    input,
  });
}

export function getAssetGraph(
  serializedGraph: any,
  prevAssetGraph?: AssetGraph,
): {
  assetGraph: AssetGraph;
  changedAssets: Map<string, Asset>;
} {
  let graph: AssetGraph;

  let reuseEdges = false;

  if (prevAssetGraph && serializedGraph.safeToSkipBundling) {
    graph = new AssetGraph({
      _contentKeyToNodeId: prevAssetGraph._contentKeyToNodeId,
      _nodeIdToContentKey: prevAssetGraph._nodeIdToContentKey,
      nodes: prevAssetGraph.nodes,
      rootNodeId: prevAssetGraph.rootNodeId,
      adjacencyList: prevAssetGraph.adjacencyList,
    });
    reuseEdges = true;
  } else if (
    prevAssetGraph &&
    (serializedGraph.updates.length > 0 || serializedGraph.nodes.length > 0)
  ) {
    graph = new AssetGraph({
      _contentKeyToNodeId: prevAssetGraph._contentKeyToNodeId,
      _nodeIdToContentKey: prevAssetGraph._nodeIdToContentKey,
      nodes: prevAssetGraph.nodes,
      initialCapacity: serializedGraph.edges.length,
      // Accomodate the root node
      initialNodeCapacity: prevAssetGraph.nodes.length + 1,
      rootNodeId: prevAssetGraph.rootNodeId,
    });
    graph.safeToIncrementallyBundle = false;
  } else {
    graph = new AssetGraph({
      _contentKeyToNodeId: new Map(),
      _nodeIdToContentKey: new Map(),
      initialCapacity: serializedGraph.edges.length,
      // Accomodate the root node
      initialNodeCapacity: serializedGraph.nodes.length + 1,
    });

    let rootNodeId = graph.addNodeByContentKey('@@root', {
      id: '@@root',
      type: 'root',
      value: null,
    });

    graph.setRootNodeId(rootNodeId);
    graph.safeToIncrementallyBundle = false;
  }

  invariant(graph, 'Asset graph not initialized');
  invariant(graph.rootNodeId != null, 'Asset graph has no root node');

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
      envId = envs.size.toString();
      envs.set(envKey, envId);
    }

    return envId;
  };

  function updateNode(newNode: AssetGraphNode, isUpdateNode: boolean) {
    if (isUpdateNode) {
      let existingNode = graph.getNodeByContentKey(newNode.id);

      assert(existingNode && existingNode.type === newNode.type);

      Object.assign(existingNode, newNode);
    } else {
      graph.addNodeByContentKey(newNode.id, newNode);
    }
  }

  let nodeTypeSwitchoverIndex = serializedGraph.nodes.length;
  let nodesCount =
    serializedGraph.nodes.length + serializedGraph.updates.length;

  for (let index = 0; index < nodesCount; index++) {
    let isUpdateNode = index >= nodeTypeSwitchoverIndex;
    let node = isUpdateNode
      ? serializedGraph.updates[index - nodeTypeSwitchoverIndex]
      : serializedGraph.nodes[index];

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

      let assetNode: AssetNode = {
        id,
        type: 'asset',
        usedSymbols: new Set(),
        usedSymbolsDownDirty: true,
        usedSymbolsUpDirty: true,
        value: asset,
      };
      updateNode(assetNode, isUpdateNode);
    } else if (node.type === 'dependency') {
      let {dependency, id} = node.value;

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

      let depNode: DependencyNode = {
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
      };

      updateNode(depNode, isUpdateNode);
    }
  }

  if (!reuseEdges) {
    for (let i = 0; i < serializedGraph.edges.length; i += 2) {
      let from = serializedGraph.edges[i];
      let to = serializedGraph.edges[i + 1];
      let fromNode = graph.getNode(from);
      let toNode = graph.getNode(to);

      if (fromNode?.type === 'dependency') {
        invariant(toNode?.type === 'asset');
      }

      if (fromNode?.type === 'asset' && toNode?.type === 'dependency') {
        fromNode.value.dependencies.set(toNode.value.id, toNode.value);
      }

      graph.addEdge(from, to);
    }
  }

  return {
    assetGraph: graph,
    changedAssets,
  };
}
