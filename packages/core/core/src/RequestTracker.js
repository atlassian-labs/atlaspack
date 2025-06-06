// @flow strict-local

import invariant, {AssertionError} from 'assert';
import path from 'path';

import {deserialize, serialize} from '@atlaspack/build-cache';
import {LMDBLiteCache, type Cache} from '@atlaspack/cache';
import {getFeatureFlag} from '@atlaspack/feature-flags';
import {ContentGraph} from '@atlaspack/graph';
import type {
  ContentGraphOpts,
  ContentKey,
  NodeId,
  SerializedContentGraph,
  Graph,
} from '@atlaspack/graph';
import logger, {instrument} from '@atlaspack/logger';
import {hashString} from '@atlaspack/rust';
import type {Async, EnvMap} from '@atlaspack/types';
import {
  type Deferred,
  isGlobMatch,
  isDirectoryInside,
  makeDeferredWithPromise,
  PromiseQueue,
} from '@atlaspack/utils';
import type {Options as WatcherOptions, Event} from '@parcel/watcher';
import type WorkerFarm from '@atlaspack/workers';
import nullthrows from 'nullthrows';

import {
  ATLASPACK_VERSION,
  VALID,
  INITIAL_BUILD,
  FILE_CREATE,
  FILE_UPDATE,
  FILE_DELETE,
  ENV_CHANGE,
  OPTION_CHANGE,
  STARTUP,
  ERROR,
} from './constants';
import type {AtlaspackV3} from './atlaspack-v3/AtlaspackV3';
import {
  type ProjectPath,
  fromProjectPathRelative,
  toProjectPathUnsafe,
  toProjectPath,
} from './projectPath';
import {report} from './ReporterRunner';
import {getConfigKeyContentHash} from './requests/ConfigRequest';
import type {AssetGraphRequestResult} from './requests/AssetGraphRequest';
import type {PackageRequestResult} from './requests/PackageRequest';
import type {ConfigRequestResult} from './requests/ConfigRequest';
import type {DevDepRequestResult} from './requests/DevDepRequest';
import type {WriteBundlesRequestResult} from './requests/WriteBundlesRequest';
import type {WriteBundleRequestResult} from './requests/WriteBundleRequest';
import type {TargetRequestResult} from './requests/TargetRequest';
import type {PathRequestResult} from './requests/PathRequest';
import type {AtlaspackConfigRequestResult} from './requests/AtlaspackConfigRequest';
import type {AtlaspackBuildRequestResult} from './requests/AtlaspackBuildRequest';
import type {EntryRequestResult} from './requests/EntryRequest';
import type {BundleGraphResult} from './requests/BundleGraphRequest';
import type {
  AssetRequestResult,
  AtlaspackOptions,
  RequestInvalidation,
  InternalFileCreateInvalidation,
  InternalGlob,
} from './types';
import {BuildAbortError, assertSignalNotAborted, hashFromOption} from './utils';
import {performance} from 'perf_hooks';

export const requestGraphEdgeTypes = {
  subrequest: 2,
  invalidated_by_update: 3,
  invalidated_by_delete: 4,
  invalidated_by_create: 5,
  invalidated_by_create_above: 6,
  dirname: 7,
};

class FSBailoutError extends Error {
  name: string = 'FSBailoutError';
}

export type RequestGraphEdgeType = $Values<typeof requestGraphEdgeTypes>;

type RequestGraphOpts = {|
  ...ContentGraphOpts<RequestGraphNode, RequestGraphEdgeType>,
  invalidNodeIds: Set<NodeId>,
  incompleteNodeIds: Set<NodeId>,
  globNodeIds: Set<NodeId>,
  envNodeIds: Set<NodeId>,
  optionNodeIds: Set<NodeId>,
  unpredicatableNodeIds: Set<NodeId>,
  invalidateOnBuildNodeIds: Set<NodeId>,
  cachedRequestChunks: Set<number>,
  configKeyNodes: Map<ProjectPath, Set<NodeId>>,
|};

type SerializedRequestGraph = {|
  ...SerializedContentGraph<RequestGraphNode, RequestGraphEdgeType>,
  invalidNodeIds: Set<NodeId>,
  incompleteNodeIds: Set<NodeId>,
  globNodeIds: Set<NodeId>,
  envNodeIds: Set<NodeId>,
  optionNodeIds: Set<NodeId>,
  unpredicatableNodeIds: Set<NodeId>,
  invalidateOnBuildNodeIds: Set<NodeId>,
  cachedRequestChunks: Set<number>,
  configKeyNodes: Map<ProjectPath, Set<NodeId>>,
|};

const FILE: 0 = 0;
const REQUEST: 1 = 1;
const FILE_NAME: 2 = 2;
const ENV: 3 = 3;
const OPTION: 4 = 4;
const GLOB: 5 = 5;
const CONFIG_KEY: 6 = 6;

type FileNode = {|id: ContentKey, +type: typeof FILE|};

type GlobNode = {|id: ContentKey, +type: typeof GLOB, value: InternalGlob|};

type FileNameNode = {|
  id: ContentKey,
  +type: typeof FILE_NAME,
|};

type EnvNode = {|
  id: ContentKey,
  +type: typeof ENV,
  value: string | void,
|};

type OptionNode = {|
  id: ContentKey,
  +type: typeof OPTION,
  hash: string,
|};

type ConfigKeyNode = {|
  id: ContentKey,
  +type: typeof CONFIG_KEY,
  configKey: string[],
  contentHash: string,
|};

type Request<TInput, TResult> = {|
  id: string,
  +type: RequestType,
  input: TInput,
  run: ({|input: TInput, ...StaticRunOpts<TResult>|}) => Async<TResult>,
|};

export type RequestResult =
  | AssetGraphRequestResult
  | PackageRequestResult
  | ConfigRequestResult
  | DevDepRequestResult
  | WriteBundlesRequestResult
  | WriteBundleRequestResult
  | TargetRequestResult
  | PathRequestResult
  | AtlaspackConfigRequestResult
  | AtlaspackBuildRequestResult
  | EntryRequestResult
  | BundleGraphResult
  | AssetRequestResult;

type InvalidateReason = number;
type RequestNode = {|
  id: ContentKey,
  +type: typeof REQUEST,
  +requestType: RequestType,
  invalidateReason: InvalidateReason,
  result?: RequestResult,
  resultCacheKey?: ?string,
  hash?: string,
|};

export const requestTypes = {
  atlaspack_build_request: 1,
  bundle_graph_request: 2,
  asset_graph_request: 3,
  entry_request: 4,
  target_request: 5,
  atlaspack_config_request: 6,
  path_request: 7,
  dev_dep_request: 8,
  asset_request: 9,
  config_request: 10,
  write_bundles_request: 11,
  package_request: 12,
  write_bundle_request: 13,
  validation_request: 14,
};

type RequestType = $Values<typeof requestTypes>;
type RequestTypeName = $Keys<typeof requestTypes>;

type RequestGraphNode =
  | RequestNode
  | FileNode
  | GlobNode
  | FileNameNode
  | EnvNode
  | OptionNode
  | ConfigKeyNode;

export type RunAPI<TResult: RequestResult> = {|
  invalidateOnFileCreate: (InternalFileCreateInvalidation) => void,
  invalidateOnFileDelete: (ProjectPath) => void,
  invalidateOnFileUpdate: (ProjectPath) => void,
  invalidateOnConfigKeyChange: (
    filePath: ProjectPath,
    configKey: string[],
    contentHash: string,
  ) => void,
  invalidateOnStartup: () => void,
  invalidateOnBuild: () => void,
  invalidateOnEnvChange: (string) => void,
  invalidateOnOptionChange: (string) => void,
  getInvalidations(): Array<RequestInvalidation>,
  storeResult(result: TResult, cacheKey?: string): void,
  getRequestResult<T: RequestResult>(contentKey: ContentKey): Async<?T>,
  getPreviousResult<T: RequestResult>(ifMatch?: string): Async<?T>,
  getSubRequests(): Array<RequestNode>,
  getInvalidSubRequests(): Array<RequestNode>,
  canSkipSubrequest(ContentKey): boolean,
  runRequest: <TInput, TResult: RequestResult>(
    subRequest: Request<TInput, TResult>,
    opts?: RunRequestOpts,
  ) => Promise<TResult>,
|};

type RunRequestOpts = {|
  force: boolean,
|};

export type StaticRunOpts<TResult> = {|
  api: RunAPI<TResult>,
  farm: WorkerFarm,
  invalidateReason: InvalidateReason,
  options: AtlaspackOptions,
  rustAtlaspack: ?AtlaspackV3,
|};

const nodeFromFilePath = (filePath: ProjectPath): RequestGraphNode => ({
  id: fromProjectPathRelative(filePath),
  type: FILE,
});
const nodeFromGlob = (glob: InternalGlob): RequestGraphNode => ({
  id: fromProjectPathRelative(glob),
  type: GLOB,
  value: glob,
});
const nodeFromFileName = (fileName: string): RequestGraphNode => ({
  id: 'file_name:' + fileName,
  type: FILE_NAME,
});

const nodeFromRequest = (request: RequestNode): RequestGraphNode => ({
  id: request.id,
  type: REQUEST,
  requestType: request.requestType,
  invalidateReason: INITIAL_BUILD,
});

const nodeFromEnv = (env: string, value: string | void): RequestGraphNode => ({
  id: 'env:' + env,
  type: ENV,
  value,
});

const nodeFromOption = (option: string, value: mixed): RequestGraphNode => ({
  id: 'option:' + option,
  type: OPTION,
  hash: hashFromOption(value),
});

const nodeFromConfigKey = (
  fileName: ProjectPath,
  configKey: string[],
  contentHash: string,
): RequestGraphNode => ({
  id: `config_key:${fromProjectPathRelative(fileName)}:${JSON.stringify(
    configKey,
  )}`,
  type: CONFIG_KEY,
  configKey,
  contentHash,
});

const keyFromEnvContentKey = (contentKey: ContentKey): string =>
  contentKey.slice('env:'.length);

const keyFromOptionContentKey = (contentKey: ContentKey): string =>
  contentKey.slice('option:'.length);

// This constant is chosen by local profiling the time to serialise n nodes and tuning until an average time of ~50 ms per blob.
// The goal is to free up the event loop periodically to allow interruption by the user.
const NODES_PER_BLOB = 2 ** 14;

export class RequestGraph extends ContentGraph<
  RequestGraphNode,
  RequestGraphEdgeType,
> {
  invalidNodeIds: Set<NodeId> = new Set();
  incompleteNodeIds: Set<NodeId> = new Set();
  incompleteNodePromises: Map<NodeId, Promise<boolean>> = new Map();
  globNodeIds: Set<NodeId> = new Set();
  envNodeIds: Set<NodeId> = new Set();
  optionNodeIds: Set<NodeId> = new Set();
  // Unpredictable nodes are requests that cannot be predicted whether they should rerun based on
  // filesystem changes alone. They should rerun on each startup of Atlaspack.
  unpredicatableNodeIds: Set<NodeId> = new Set();
  invalidateOnBuildNodeIds: Set<NodeId> = new Set();
  cachedRequestChunks: Set<number> = new Set();
  configKeyNodes: Map<ProjectPath, Set<NodeId>> = new Map();
  nodesPerBlob: number = NODES_PER_BLOB;

  // $FlowFixMe[prop-missing]
  static deserialize(opts: RequestGraphOpts): RequestGraph {
    // $FlowFixMe[prop-missing]
    let deserialized = new RequestGraph(opts);
    deserialized.invalidNodeIds = opts.invalidNodeIds;
    deserialized.incompleteNodeIds = opts.incompleteNodeIds;
    deserialized.globNodeIds = opts.globNodeIds;
    deserialized.envNodeIds = opts.envNodeIds;
    deserialized.optionNodeIds = opts.optionNodeIds;
    deserialized.unpredicatableNodeIds = opts.unpredicatableNodeIds;
    deserialized.invalidateOnBuildNodeIds = opts.invalidateOnBuildNodeIds;
    deserialized.cachedRequestChunks = opts.cachedRequestChunks;
    deserialized.configKeyNodes = opts.configKeyNodes;
    return deserialized;
  }

  // $FlowFixMe[prop-missing]
  serialize(): SerializedRequestGraph {
    return {
      ...super.serialize(),
      invalidNodeIds: this.invalidNodeIds,
      incompleteNodeIds: this.incompleteNodeIds,
      globNodeIds: this.globNodeIds,
      envNodeIds: this.envNodeIds,
      optionNodeIds: this.optionNodeIds,
      unpredicatableNodeIds: this.unpredicatableNodeIds,
      invalidateOnBuildNodeIds: this.invalidateOnBuildNodeIds,
      cachedRequestChunks: this.cachedRequestChunks,
      configKeyNodes: this.configKeyNodes,
    };
  }

  // addNode for RequestGraph should not override the value if added multiple times
  addNode(node: RequestGraphNode): NodeId {
    let nodeId = this._contentKeyToNodeId.get(node.id);
    if (nodeId != null) {
      return nodeId;
    }

    nodeId = super.addNodeByContentKey(node.id, node);
    if (node.type === GLOB) {
      this.globNodeIds.add(nodeId);
    } else if (node.type === ENV) {
      this.envNodeIds.add(nodeId);
    } else if (node.type === OPTION) {
      this.optionNodeIds.add(nodeId);
    }

    this.removeCachedRequestChunkForNode(nodeId);

    return nodeId;
  }

  removeNode(nodeId: NodeId, removeOrphans: boolean = true): void {
    this.invalidNodeIds.delete(nodeId);
    this.incompleteNodeIds.delete(nodeId);
    this.incompleteNodePromises.delete(nodeId);
    this.unpredicatableNodeIds.delete(nodeId);
    this.invalidateOnBuildNodeIds.delete(nodeId);
    let node = nullthrows(this.getNode(nodeId));
    if (node.type === GLOB) {
      this.globNodeIds.delete(nodeId);
    } else if (node.type === ENV) {
      this.envNodeIds.delete(nodeId);
    } else if (node.type === OPTION) {
      this.optionNodeIds.delete(nodeId);
    } else if (node.type === CONFIG_KEY) {
      for (let configKeyNodes of this.configKeyNodes.values()) {
        configKeyNodes.delete(nodeId);
      }
    }
    return super.removeNode(nodeId, removeOrphans);
  }

  getRequestNode(nodeId: NodeId): RequestNode {
    let node = nullthrows(this.getNode(nodeId));

    if (node.type === REQUEST) {
      return node;
    }

    throw new AssertionError({
      message: `Expected a request node: ${
        node.type
      } (${typeof node.type}) does not equal ${REQUEST} (${typeof REQUEST}).`,
      expected: REQUEST,
      actual: node.type,
    });
  }

  replaceSubrequests(
    requestNodeId: NodeId,
    subrequestContentKeys: Array<ContentKey>,
  ) {
    let subrequestNodeIds = [];
    for (let key of subrequestContentKeys) {
      if (this.hasContentKey(key)) {
        subrequestNodeIds.push(this.getNodeIdByContentKey(key));
      }
    }

    this.replaceNodeIdsConnectedTo(
      requestNodeId,
      subrequestNodeIds,
      null,
      requestGraphEdgeTypes.subrequest,
    );
  }

  invalidateNode(nodeId: NodeId, reason: InvalidateReason) {
    let node = nullthrows(this.getNode(nodeId));
    invariant(node.type === REQUEST);
    node.invalidateReason |= reason;
    this.invalidNodeIds.add(nodeId);

    let parentNodes = this.getNodeIdsConnectedTo(
      nodeId,
      requestGraphEdgeTypes.subrequest,
    );
    for (let parentNode of parentNodes) {
      this.invalidateNode(parentNode, reason);
    }

    // If the node is invalidated, the cached request chunk on disk needs to be re-written
    this.removeCachedRequestChunkForNode(nodeId);
  }

  /**
   * Nodes that are invalidated on start-up, such as JavaScript babel configuration files which are
   * imported when the build kicks-off and might doing arbitrary work such as reading from the file
   * system.
   */
  invalidateUnpredictableNodes() {
    for (let nodeId of this.unpredicatableNodeIds) {
      let node = nullthrows(this.getNode(nodeId));
      invariant(node.type !== FILE && node.type !== GLOB);
      this.invalidateNode(nodeId, STARTUP);
    }
  }

  /**
   * Effectively uncacheable nodes.
   */
  invalidateOnBuildNodes() {
    for (let nodeId of this.invalidateOnBuildNodeIds) {
      let node = nullthrows(this.getNode(nodeId));
      invariant(node.type !== FILE && node.type !== GLOB);
      this.invalidateNode(nodeId, STARTUP);
    }
  }

  /**
   * Nodes invalidated by environment changes, corresponds to `env: ...` inputs.
   */
  invalidateEnvNodes(env: EnvMap): string[] {
    const invalidatedKeys = [];

    for (let nodeId of this.envNodeIds) {
      let node = nullthrows(this.getNode(nodeId));
      invariant(node.type === ENV);

      const key = keyFromEnvContentKey(node.id);
      if (env[key] !== node.value) {
        invalidatedKeys.push(key);

        let parentNodes = this.getNodeIdsConnectedTo(
          nodeId,
          requestGraphEdgeTypes.invalidated_by_update,
        );
        for (let parentNode of parentNodes) {
          this.invalidateNode(parentNode, ENV_CHANGE);
        }
      }
    }

    return invalidatedKeys;
  }

  /**
   * Nodes invalidated by option changes.
   */
  invalidateOptionNodes(options: AtlaspackOptions): string[] {
    const invalidatedKeys = [];

    for (let nodeId of this.optionNodeIds) {
      let node = nullthrows(this.getNode(nodeId));
      invariant(node.type === OPTION);
      const key = keyFromOptionContentKey(node.id);

      if (hashFromOption(options[key]) !== node.hash) {
        invalidatedKeys.push(key);
        let parentNodes = this.getNodeIdsConnectedTo(
          nodeId,
          requestGraphEdgeTypes.invalidated_by_update,
        );
        for (let parentNode of parentNodes) {
          this.invalidateNode(parentNode, OPTION_CHANGE);
        }
      }
    }

    return invalidatedKeys;
  }

  invalidateOnConfigKeyChange(
    requestNodeId: NodeId,
    filePath: ProjectPath,
    configKey: string[],
    contentHash: string,
  ) {
    let configKeyNodeId = this.addNode(
      nodeFromConfigKey(filePath, configKey, contentHash),
    );
    let nodes = this.configKeyNodes.get(filePath);

    if (!nodes) {
      nodes = new Set();
      this.configKeyNodes.set(filePath, nodes);
    }

    nodes.add(configKeyNodeId);

    if (
      !this.hasEdge(
        requestNodeId,
        configKeyNodeId,
        requestGraphEdgeTypes.invalidated_by_update,
      )
    ) {
      this.addEdge(
        requestNodeId,
        configKeyNodeId,
        // Store as an update edge, but file deletes are handled too
        requestGraphEdgeTypes.invalidated_by_update,
      );
    }
  }

  invalidateOnFileUpdate(requestNodeId: NodeId, filePath: ProjectPath) {
    let fileNodeId = this.addNode(nodeFromFilePath(filePath));

    if (
      !this.hasEdge(
        requestNodeId,
        fileNodeId,
        requestGraphEdgeTypes.invalidated_by_update,
      )
    ) {
      this.addEdge(
        requestNodeId,
        fileNodeId,
        requestGraphEdgeTypes.invalidated_by_update,
      );
    }
  }

  invalidateOnFileDelete(requestNodeId: NodeId, filePath: ProjectPath) {
    let fileNodeId = this.addNode(nodeFromFilePath(filePath));

    if (
      !this.hasEdge(
        requestNodeId,
        fileNodeId,
        requestGraphEdgeTypes.invalidated_by_delete,
      )
    ) {
      this.addEdge(
        requestNodeId,
        fileNodeId,
        requestGraphEdgeTypes.invalidated_by_delete,
      );
    }
  }

  invalidateOnFileCreate(
    requestNodeId: NodeId,
    input: InternalFileCreateInvalidation,
  ) {
    let node;
    if (input.glob != null) {
      node = nodeFromGlob(input.glob);
    } else if (input.fileName != null && input.aboveFilePath != null) {
      let aboveFilePath = input.aboveFilePath;

      // Create nodes and edges for each part of the filename pattern.
      // For example, 'node_modules/foo' would create two nodes and one edge.
      // This creates a sort of trie structure within the graph that can be
      // quickly matched by following the edges. This is also memory efficient
      // since common sub-paths (e.g. 'node_modules') are deduplicated.
      let parts = input.fileName.split('/').reverse();
      let lastNodeId;
      for (let part of parts) {
        let fileNameNode = nodeFromFileName(part);

        let fileNameNodeId = this.addNode(fileNameNode);
        if (
          lastNodeId != null &&
          !this.hasEdge(
            lastNodeId,
            fileNameNodeId,
            requestGraphEdgeTypes.dirname,
          )
        ) {
          this.addEdge(
            lastNodeId,
            fileNameNodeId,
            requestGraphEdgeTypes.dirname,
          );
        }

        lastNodeId = fileNameNodeId;
      }

      // The `aboveFilePath` condition asserts that requests are only invalidated
      // if the file being created is "above" it in the filesystem (e.g. the file
      // is created in a parent directory). There is likely to already be a node
      // for this file in the graph (e.g. the source file) that we can reuse for this.
      node = nodeFromFilePath(aboveFilePath);
      let nodeId = this.addNode(node);

      // Now create an edge from the `aboveFilePath` node to the first file_name node
      // in the chain created above, and an edge from the last node in the chain back to
      // the `aboveFilePath` node. When matching, we will start from the first node in
      // the chain, and continue following it to parent directories until there is an
      // edge pointing an `aboveFilePath` node that also points to the start of the chain.
      // This indicates a complete match, and any requests attached to the `aboveFilePath`
      // node will be invalidated.
      let firstId = 'file_name:' + parts[0];
      let firstNodeId = this.getNodeIdByContentKey(firstId);
      if (
        !this.hasEdge(
          nodeId,
          firstNodeId,
          requestGraphEdgeTypes.invalidated_by_create_above,
        )
      ) {
        this.addEdge(
          nodeId,
          firstNodeId,
          requestGraphEdgeTypes.invalidated_by_create_above,
        );
      }

      invariant(lastNodeId != null);
      if (
        !this.hasEdge(
          lastNodeId,
          nodeId,
          requestGraphEdgeTypes.invalidated_by_create_above,
        )
      ) {
        this.addEdge(
          lastNodeId,
          nodeId,
          requestGraphEdgeTypes.invalidated_by_create_above,
        );
      }
    } else if (input.filePath != null) {
      node = nodeFromFilePath(input.filePath);
    } else {
      throw new Error('Invalid invalidation');
    }

    let nodeId = this.addNode(node);
    if (
      !this.hasEdge(
        requestNodeId,
        nodeId,
        requestGraphEdgeTypes.invalidated_by_create,
      )
    ) {
      this.addEdge(
        requestNodeId,
        nodeId,
        requestGraphEdgeTypes.invalidated_by_create,
      );
    }
  }

  invalidateOnStartup(requestNodeId: NodeId) {
    this.getRequestNode(requestNodeId);
    this.unpredicatableNodeIds.add(requestNodeId);
  }

  invalidateOnBuild(requestNodeId: NodeId) {
    this.getRequestNode(requestNodeId);
    this.invalidateOnBuildNodeIds.add(requestNodeId);
  }

  invalidateOnEnvChange(
    requestNodeId: NodeId,
    env: string,
    value: string | void,
  ) {
    let envNode = nodeFromEnv(env, value);
    let envNodeId = this.addNode(envNode);

    if (
      !this.hasEdge(
        requestNodeId,
        envNodeId,
        requestGraphEdgeTypes.invalidated_by_update,
      )
    ) {
      this.addEdge(
        requestNodeId,
        envNodeId,
        requestGraphEdgeTypes.invalidated_by_update,
      );
    }
  }

  invalidateOnOptionChange(
    requestNodeId: NodeId,
    option: string,
    value: mixed,
  ) {
    let optionNode = nodeFromOption(option, value);
    let optionNodeId = this.addNode(optionNode);

    if (
      !this.hasEdge(
        requestNodeId,
        optionNodeId,
        requestGraphEdgeTypes.invalidated_by_update,
      )
    ) {
      this.addEdge(
        requestNodeId,
        optionNodeId,
        requestGraphEdgeTypes.invalidated_by_update,
      );
    }
  }

  clearInvalidations(nodeId: NodeId) {
    this.unpredicatableNodeIds.delete(nodeId);
    this.invalidateOnBuildNodeIds.delete(nodeId);
    this.replaceNodeIdsConnectedTo(
      nodeId,
      [],
      null,
      requestGraphEdgeTypes.invalidated_by_update,
    );
    this.replaceNodeIdsConnectedTo(
      nodeId,
      [],
      null,
      requestGraphEdgeTypes.invalidated_by_delete,
    );
    this.replaceNodeIdsConnectedTo(
      nodeId,
      [],
      null,
      requestGraphEdgeTypes.invalidated_by_create,
    );
  }

  getInvalidations(requestNodeId: NodeId): Array<RequestInvalidation> {
    if (!this.hasNode(requestNodeId)) {
      return [];
    }

    // For now just handling updates. Could add creates/deletes later if needed.
    let invalidations = this.getNodeIdsConnectedFrom(
      requestNodeId,
      requestGraphEdgeTypes.invalidated_by_update,
    );
    return invalidations
      .map((nodeId) => {
        let node = nullthrows(this.getNode(nodeId));
        switch (node.type) {
          case FILE:
            return {type: 'file', filePath: toProjectPathUnsafe(node.id)};
          case ENV:
            return {type: 'env', key: keyFromEnvContentKey(node.id)};
          case OPTION:
            return {
              type: 'option',
              key: keyFromOptionContentKey(node.id),
            };
        }
      })
      .filter(Boolean);
  }

  getSubRequests(requestNodeId: NodeId): Array<RequestNode> {
    if (!this.hasNode(requestNodeId)) {
      return [];
    }

    let subRequests = this.getNodeIdsConnectedFrom(
      requestNodeId,
      requestGraphEdgeTypes.subrequest,
    );

    return subRequests.map((nodeId) => {
      let node = nullthrows(this.getNode(nodeId));
      invariant(node.type === REQUEST);
      return node;
    });
  }

  getInvalidSubRequests(requestNodeId: NodeId): Array<RequestNode> {
    if (!this.hasNode(requestNodeId)) {
      return [];
    }

    let subRequests = this.getNodeIdsConnectedFrom(
      requestNodeId,
      requestGraphEdgeTypes.subrequest,
    );

    return subRequests
      .filter((id) => this.invalidNodeIds.has(id))
      .map((nodeId) => {
        let node = nullthrows(this.getNode(nodeId));
        invariant(node.type === REQUEST);
        return node;
      });
  }

  invalidateFileNameNode(
    node: FileNameNode,
    filePath: ProjectPath,
    matchNodes: Array<FileNode>,
    invalidateNode: (NodeId, InvalidateReason) => void,
  ) {
    // If there is an edge between this file_name node and one of the original file nodes pointed to
    // by the original file_name node, and the matched node is inside the current directory, invalidate
    // all connected requests pointed to by the file node.

    let nodeId = this.getNodeIdByContentKey(node.id);
    let dirname = path.dirname(fromProjectPathRelative(filePath));

    if (getFeatureFlag('fixQuadraticCacheInvalidation')) {
      while (dirname !== '/') {
        if (!this.hasContentKey(dirname)) break;
        const matchNodeId = this.getNodeIdByContentKey(dirname);
        if (
          !this.hasEdge(
            nodeId,
            matchNodeId,
            requestGraphEdgeTypes.invalidated_by_create_above,
          )
        )
          break;

        const connectedNodes = this.getNodeIdsConnectedTo(
          matchNodeId,
          requestGraphEdgeTypes.invalidated_by_create,
        );
        for (let connectedNode of connectedNodes) {
          invalidateNode(connectedNode, FILE_CREATE);
        }

        dirname = path.dirname(dirname);
      }
    } else {
      for (let matchNode of matchNodes) {
        let matchNodeId = this.getNodeIdByContentKey(matchNode.id);
        if (
          this.hasEdge(
            nodeId,
            matchNodeId,
            requestGraphEdgeTypes.invalidated_by_create_above,
          ) &&
          isDirectoryInside(
            fromProjectPathRelative(toProjectPathUnsafe(matchNode.id)),
            dirname,
          )
        ) {
          let connectedNodes = this.getNodeIdsConnectedTo(
            matchNodeId,
            requestGraphEdgeTypes.invalidated_by_create,
          );
          for (let connectedNode of connectedNodes) {
            this.invalidateNode(connectedNode, FILE_CREATE);
          }
        }
      }
    }

    // Find the `file_name` node for the parent directory and
    // recursively invalidate connected requests as described above.
    let basename = path.basename(dirname);
    let contentKey = 'file_name:' + basename;
    if (this.hasContentKey(contentKey)) {
      if (
        this.hasEdge(
          nodeId,
          this.getNodeIdByContentKey(contentKey),
          requestGraphEdgeTypes.dirname,
        )
      ) {
        let parent = nullthrows(this.getNodeByContentKey(contentKey));
        invariant(parent.type === FILE_NAME);
        this.invalidateFileNameNode(
          parent,
          toProjectPathUnsafe(dirname),
          matchNodes,
          invalidateNode,
        );
      }
    }
  }

  async respondToFSEvents(
    events: Array<Event>,
    options: AtlaspackOptions,
    threshold: number,
    /**
     * True if this is the start-up (loading phase) invalidation.
     */
    isInitialBuild: boolean = false,
  ): Promise<{|
    didInvalidate: boolean,
    invalidationsByPath: Map<string, number>,
  |}> {
    let didInvalidate = false;
    let count = 0;
    let predictedTime = 0;
    let startTime = Date.now();
    const enableOptimization = getFeatureFlag('fixQuadraticCacheInvalidation');
    const removeOrphans = !enableOptimization;

    const invalidatedNodes = new Set();
    const invalidateNode = (nodeId, reason) => {
      if (enableOptimization && invalidatedNodes.has(nodeId)) {
        return;
      }
      invalidatedNodes.add(nodeId);
      this.invalidateNode(nodeId, reason);
    };
    const aboveCache = new Map();
    const getAbove = (fileNameNodeId) => {
      const cachedResult = aboveCache.get(fileNameNodeId);
      if (enableOptimization && cachedResult) {
        return cachedResult;
      }

      let above = [];
      const children = this.getNodeIdsConnectedTo(
        fileNameNodeId,
        requestGraphEdgeTypes.invalidated_by_create_above,
      );
      for (const nodeId of children) {
        let node = nullthrows(this.getNode(nodeId));
        if (node.type === FILE) {
          above.push(node);
        }
      }
      aboveCache.set(fileNameNodeId, above);
      return above;
    };

    const invalidationsByPath = new Map();
    for (let {path: _path, type} of events) {
      const invalidationsBefore = this.getInvalidNodeCount();

      if (
        !enableOptimization &&
        process.env.ATLASPACK_DISABLE_CACHE_TIMEOUT !== 'true' &&
        ++count === 256
      ) {
        let duration = Date.now() - startTime;
        predictedTime = duration * (events.length >> 8);
        if (predictedTime > threshold) {
          logger.warn({
            origin: '@atlaspack/core',
            message:
              'Building with clean cache. Cache invalidation took too long.',
            meta: {
              trackableEvent: 'cache_invalidation_timeout',
              watcherEventCount: events.length,
              predictedTime,
            },
          });
          throw new FSBailoutError(
            'Responding to file system events exceeded threshold, start with empty cache.',
          );
        }
      }

      let _filePath = toProjectPath(options.projectRoot, _path);
      let filePath = fromProjectPathRelative(_filePath);
      let hasFileRequest = this.hasContentKey(filePath);

      // If we see a 'create' event for the project root itself,
      // this means the project root was moved and we need to
      // re-run all requests.
      if (type === 'create' && filePath === '') {
        logger.verbose({
          origin: '@atlaspack/core',
          message:
            'Watcher reported project root create event. Invalidate all nodes.',
          meta: {
            trackableEvent: 'project_root_create',
          },
        });
        for (let [id, node] of this.nodes.entries()) {
          if (node?.type === REQUEST) {
            this.invalidNodeIds.add(id);
          }
        }
        return {
          didInvalidate: true,
          invalidationsByPath: new Map(),
        };
      }

      // sometimes mac os reports update events as create events.
      // if it was a create event, but the file already exists in the graph,
      // then also invalidate nodes connected by invalidated_by_update edges.
      if (hasFileRequest && (type === 'create' || type === 'update')) {
        let nodeId = this.getNodeIdByContentKey(filePath);
        let nodes = this.getNodeIdsConnectedTo(
          nodeId,
          requestGraphEdgeTypes.invalidated_by_update,
        );

        for (let connectedNode of nodes) {
          didInvalidate = true;
          invalidateNode(connectedNode, FILE_UPDATE);
        }

        if (type === 'create') {
          let nodes = this.getNodeIdsConnectedTo(
            nodeId,
            requestGraphEdgeTypes.invalidated_by_create,
          );
          for (let connectedNode of nodes) {
            didInvalidate = true;
            invalidateNode(connectedNode, FILE_CREATE);
          }
        }
      } else if (type === 'create') {
        let basename = path.basename(filePath);
        let fileNameNode = this.getNodeByContentKey('file_name:' + basename);
        if (fileNameNode != null && fileNameNode.type === FILE_NAME) {
          let fileNameNodeId = this.getNodeIdByContentKey(
            'file_name:' + basename,
          );

          // Find potential file nodes to be invalidated if this file name pattern matches
          let above: Array<FileNode> = getAbove(fileNameNodeId);
          if (above.length > 0) {
            didInvalidate = true;
            this.invalidateFileNameNode(
              fileNameNode,
              _filePath,
              above,
              invalidateNode,
            );
          }
        }

        for (let globeNodeId of this.globNodeIds) {
          let globNode = this.getNode(globeNodeId);
          invariant(globNode && globNode.type === GLOB);

          if (isGlobMatch(filePath, fromProjectPathRelative(globNode.value))) {
            let connectedNodes = this.getNodeIdsConnectedTo(
              globeNodeId,
              requestGraphEdgeTypes.invalidated_by_create,
            );
            for (let connectedNode of connectedNodes) {
              didInvalidate = true;
              invalidateNode(connectedNode, FILE_CREATE);
            }
          }
        }
      } else if (hasFileRequest && type === 'delete') {
        let nodeId = this.getNodeIdByContentKey(filePath);
        for (let connectedNode of this.getNodeIdsConnectedTo(
          nodeId,
          requestGraphEdgeTypes.invalidated_by_delete,
        )) {
          didInvalidate = true;
          invalidateNode(connectedNode, FILE_DELETE);
        }

        // Delete the file node since it doesn't exist anymore.
        // This ensures that files that don't exist aren't sent
        // to requests as invalidations for future requests.
        this.removeNode(nodeId, removeOrphans);
      }

      let configKeyNodes = this.configKeyNodes.get(_filePath);

      // With granular invalidations we will always run this block,
      // so even if we get a create event (for whatever reason), we will still
      // try to limit invalidations from config key changes through hashing.
      //
      // Currently create events can invalidate a large number of nodes due to
      // "create above" invalidations.
      const isConfigKeyChange =
        getFeatureFlag('granularTsConfigInvalidation') ||
        type === 'delete' ||
        type === 'update';
      if (configKeyNodes && isConfigKeyChange) {
        for (let nodeId of configKeyNodes) {
          let isInvalid = type === 'delete';

          if (type !== 'delete') {
            let node = this.getNode(nodeId);
            invariant(node && node.type === CONFIG_KEY);

            let contentHash = await getConfigKeyContentHash(
              _filePath,
              node.configKey,
              options,
            );

            isInvalid = node.contentHash !== contentHash;
          }

          if (isInvalid) {
            for (let connectedNode of this.getNodeIdsConnectedTo(
              nodeId,
              requestGraphEdgeTypes.invalidated_by_update,
            )) {
              invalidateNode(
                connectedNode,
                type === 'delete' ? FILE_DELETE : FILE_UPDATE,
              );
            }
            didInvalidate = true;
            this.removeNode(nodeId, removeOrphans);
          }
        }
      }

      const invalidationsAfter = this.getInvalidNodeCount();
      const invalidationsForEvent = invalidationsAfter - invalidationsBefore;
      invalidationsByPath.set(
        _path,
        (invalidationsByPath.get(_path) ?? 0) + invalidationsForEvent,
      );
    }

    if (getFeatureFlag('fixQuadraticCacheInvalidation')) {
      cleanUpOrphans(this);
    }

    let duration = Date.now() - startTime;
    logger.verbose({
      origin: '@atlaspack/core',
      message: `RequestGraph.respondToFSEvents duration: ${duration}`,
      meta: {
        trackableEvent: 'fsevent_response_time',
        duration,
        predictedTime,
        isInitialBuild,
        numberOfEvents: events.length,
        numberOfInvalidatedNodes: invalidatedNodes.size,
      },
    });

    return {
      didInvalidate,
      invalidationsByPath,
    };
  }

  hasCachedRequestChunk(index: number): boolean {
    return this.cachedRequestChunks.has(index);
  }

  setCachedRequestChunk(index: number): void {
    this.cachedRequestChunks.add(index);
  }

  removeCachedRequestChunkForNode(nodeId: number): void {
    this.cachedRequestChunks.delete(Math.floor(nodeId / this.nodesPerBlob));
  }

  /**
   * Returns the number of invalidated nodes in the graph.
   */
  getInvalidNodeCount(): number {
    return this.invalidNodeIds.size;
  }
}

export default class RequestTracker {
  graph: RequestGraph;
  farm: WorkerFarm;
  options: AtlaspackOptions;
  rustAtlaspack: ?AtlaspackV3;
  signal: ?AbortSignal;
  stats: Map<RequestType, number> = new Map();

  constructor({
    graph,
    farm,
    options,
    rustAtlaspack,
  }: {|
    graph?: RequestGraph,
    farm: WorkerFarm,
    options: AtlaspackOptions,
    rustAtlaspack?: AtlaspackV3,
  |}) {
    this.graph = graph || new RequestGraph();
    this.farm = farm;
    this.options = options;
    this.rustAtlaspack = rustAtlaspack;
  }

  // TODO: refactor (abortcontroller should be created by RequestTracker)
  setSignal(signal?: AbortSignal) {
    this.signal = signal;
  }

  startRequest(request: RequestNode): {|
    requestNodeId: NodeId,
    deferred: Deferred<boolean>,
  |} {
    let didPreviouslyExist = this.graph.hasContentKey(request.id);
    let requestNodeId;
    if (didPreviouslyExist) {
      requestNodeId = this.graph.getNodeIdByContentKey(request.id);
      // Clear existing invalidations for the request so that the new
      // invalidations created during the request replace the existing ones.
      this.graph.clearInvalidations(requestNodeId);
    } else {
      requestNodeId = this.graph.addNode(nodeFromRequest(request));
    }

    this.graph.incompleteNodeIds.add(requestNodeId);
    this.graph.invalidNodeIds.delete(requestNodeId);

    let {promise, deferred} = makeDeferredWithPromise();
    this.graph.incompleteNodePromises.set(requestNodeId, promise);

    return {requestNodeId, deferred};
  }

  // If a cache key is provided, the result will be removed from the node and stored in a separate cache entry
  storeResult(nodeId: NodeId, result: RequestResult, cacheKey: ?string) {
    let node = this.graph.getNode(nodeId);
    if (node && node.type === REQUEST) {
      node.result = result;
      node.resultCacheKey = cacheKey;
    }
  }

  hasValidResult(nodeId: NodeId): boolean {
    return (
      this.graph.hasNode(nodeId) &&
      !this.graph.invalidNodeIds.has(nodeId) &&
      !this.graph.incompleteNodeIds.has(nodeId)
    );
  }

  async getRequestResult<T: RequestResult>(
    contentKey: ContentKey,
    ifMatch?: string,
  ): Promise<?T> {
    let node = nullthrows(this.graph.getNodeByContentKey(contentKey));
    invariant(node.type === REQUEST);

    if (ifMatch != null && node.resultCacheKey !== ifMatch) {
      return null;
    }

    if (node.result != undefined) {
      // $FlowFixMe
      let result: T = (node.result: any);
      return result;
    } else if (node.resultCacheKey != null && ifMatch == null) {
      let key = node.resultCacheKey;
      if (!getFeatureFlag('cachePerformanceImprovements')) {
        invariant(this.options.cache.hasLargeBlob(key));
      }

      let cachedResult: T = getFeatureFlag('cachePerformanceImprovements')
        ? nullthrows(await this.options.cache.get<T>(key))
        : deserialize(await this.options.cache.getLargeBlob(key));
      node.result = cachedResult;
      return cachedResult;
    }
  }

  completeRequest(nodeId: NodeId) {
    this.graph.invalidNodeIds.delete(nodeId);
    this.graph.incompleteNodeIds.delete(nodeId);
    this.graph.incompleteNodePromises.delete(nodeId);
    let node = this.graph.getNode(nodeId);
    if (node && node.type === REQUEST) {
      node.invalidateReason = VALID;
    }
    this.graph.removeCachedRequestChunkForNode(nodeId);
  }

  rejectRequest(nodeId: NodeId) {
    this.graph.incompleteNodeIds.delete(nodeId);
    this.graph.incompleteNodePromises.delete(nodeId);

    let node = this.graph.getNode(nodeId);
    if (node?.type === REQUEST) {
      this.graph.invalidateNode(nodeId, ERROR);
    }
  }

  respondToFSEvents(
    events: Array<Event>,
    threshold: number,
  ): Promise<{|
    didInvalidate: boolean,
    invalidationsByPath: Map<string, number>,
  |}> {
    return this.graph.respondToFSEvents(events, this.options, threshold);
  }

  hasInvalidRequests(): boolean {
    return this.graph.invalidNodeIds.size > 0;
  }

  getInvalidRequests(): Array<RequestNode> {
    let invalidRequests = [];
    for (let id of this.graph.invalidNodeIds) {
      let node = nullthrows(this.graph.getNode(id));
      invariant(node.type === REQUEST);
      invalidRequests.push(node);
    }
    return invalidRequests;
  }

  replaceSubrequests(
    requestNodeId: NodeId,
    subrequestContextKeys: Array<ContentKey>,
  ) {
    this.graph.replaceSubrequests(requestNodeId, subrequestContextKeys);
  }

  async runRequest<TInput, TResult: RequestResult>(
    request: Request<TInput, TResult>,
    opts?: ?RunRequestOpts,
  ): Promise<TResult> {
    let hasKey = this.graph.hasContentKey(request.id);
    let requestId = hasKey
      ? this.graph.getNodeIdByContentKey(request.id)
      : undefined;
    let hasValidResult = requestId != null && this.hasValidResult(requestId);

    if (!opts?.force && hasValidResult) {
      // $FlowFixMe[incompatible-type]
      return this.getRequestResult<TResult>(request.id);
    }

    if (requestId != null) {
      let incompletePromise = this.graph.incompleteNodePromises.get(requestId);
      if (incompletePromise != null) {
        // There is a another instance of this request already running, wait for its completion and reuse its result
        try {
          if (await incompletePromise) {
            // $FlowFixMe[incompatible-type]
            return this.getRequestResult<TResult>(request.id);
          }
        } catch (e) {
          // Rerun this request
        }
      }
    }

    let previousInvalidations =
      requestId != null ? this.graph.getInvalidations(requestId) : [];
    let {requestNodeId, deferred} = this.startRequest({
      id: request.id,
      type: REQUEST,
      requestType: request.type,
      invalidateReason: INITIAL_BUILD,
    });

    let {api, subRequestContentKeys} = this.createAPI(
      requestNodeId,
      previousInvalidations,
    );

    try {
      let node = this.graph.getRequestNode(requestNodeId);

      this.stats.set(request.type, (this.stats.get(request.type) ?? 0) + 1);

      let result = await request.run({
        input: request.input,
        api,
        farm: this.farm,
        invalidateReason: node.invalidateReason,
        options: this.options,
        rustAtlaspack: this.rustAtlaspack,
      });

      assertSignalNotAborted(this.signal);
      this.completeRequest(requestNodeId);

      deferred.resolve(true);
      return result;
    } catch (err) {
      if (
        !(err instanceof BuildAbortError) &&
        request.type === requestTypes.dev_dep_request
      ) {
        logger.verbose({
          origin: '@atlaspack/core',
          message: `Failed DevDepRequest`,
          meta: {
            trackableEvent: 'failed_dev_dep_request',
            hasKey,
            hasValidResult,
          },
        });
      }

      this.rejectRequest(requestNodeId);
      deferred.resolve(false);
      throw err;
    } finally {
      this.graph.replaceSubrequests(requestNodeId, [...subRequestContentKeys]);
    }
  }

  flushStats(): {[requestType: string]: number} {
    let requestTypeEntries = {};

    for (let key of (Object.keys(requestTypes): RequestTypeName[])) {
      requestTypeEntries[requestTypes[key]] = key;
    }

    let formattedStats = {};

    for (let [requestType, count] of this.stats.entries()) {
      let requestTypeName = requestTypeEntries[requestType];
      formattedStats[requestTypeName] = count;
    }

    this.stats = new Map();

    return formattedStats;
  }

  createAPI<TResult: RequestResult>(
    requestId: NodeId,
    previousInvalidations: Array<RequestInvalidation>,
  ): {|api: RunAPI<TResult>, subRequestContentKeys: Set<ContentKey>|} {
    let subRequestContentKeys = new Set<ContentKey>();
    let api: RunAPI<TResult> = {
      invalidateOnFileCreate: (input) =>
        this.graph.invalidateOnFileCreate(requestId, input),
      invalidateOnConfigKeyChange: (filePath, configKey, contentHash) =>
        this.graph.invalidateOnConfigKeyChange(
          requestId,
          filePath,
          configKey,
          contentHash,
        ),
      invalidateOnFileDelete: (filePath) =>
        this.graph.invalidateOnFileDelete(requestId, filePath),
      invalidateOnFileUpdate: (filePath) =>
        this.graph.invalidateOnFileUpdate(requestId, filePath),
      invalidateOnStartup: () => this.graph.invalidateOnStartup(requestId),
      invalidateOnBuild: () => this.graph.invalidateOnBuild(requestId),
      invalidateOnEnvChange: (env) =>
        this.graph.invalidateOnEnvChange(requestId, env, this.options.env[env]),
      invalidateOnOptionChange: (option) =>
        this.graph.invalidateOnOptionChange(
          requestId,
          option,
          this.options[option],
        ),
      getInvalidations: () => previousInvalidations,
      storeResult: (result, cacheKey) => {
        this.storeResult(requestId, result, cacheKey);
      },
      getSubRequests: () => this.graph.getSubRequests(requestId),
      getInvalidSubRequests: () => this.graph.getInvalidSubRequests(requestId),
      getPreviousResult: <T: RequestResult>(ifMatch?: string): Async<?T> => {
        let contentKey = nullthrows(this.graph.getNode(requestId)?.id);
        return this.getRequestResult<T>(contentKey, ifMatch);
      },
      getRequestResult: <T: RequestResult>(id): Async<?T> =>
        this.getRequestResult<T>(id),
      canSkipSubrequest: (contentKey) => {
        if (
          this.graph.hasContentKey(contentKey) &&
          this.hasValidResult(this.graph.getNodeIdByContentKey(contentKey))
        ) {
          subRequestContentKeys.add(contentKey);
          return true;
        }

        return false;
      },
      runRequest: <TInput, TResult: RequestResult>(
        subRequest: Request<TInput, TResult>,
        opts?: RunRequestOpts,
      ): Promise<TResult> => {
        subRequestContentKeys.add(subRequest.id);
        return this.runRequest<TInput, TResult>(subRequest, opts);
      },
    };

    return {api, subRequestContentKeys};
  }

  async writeToCache(signal?: AbortSignal) {
    const options = this.options;
    async function runCacheImprovements<T>(
      newPath: (cache: LMDBLiteCache) => Promise<T>,
      oldPath: () => Promise<T>,
    ): Promise<T> {
      if (getFeatureFlag('cachePerformanceImprovements')) {
        invariant(options.cache instanceof LMDBLiteCache);
        const result = await newPath(options.cache);
        return result;
      } else {
        const result = await oldPath();
        return result;
      }
    }

    let cacheKey = getCacheKey(this.options);
    let requestGraphKey = getFeatureFlag('cachePerformanceImprovements')
      ? `${cacheKey}/RequestGraph`
      : `requestGraph-${cacheKey}`;
    let snapshotKey = getFeatureFlag('cachePerformanceImprovements')
      ? `${cacheKey}/snapshot`
      : `snapshot-${cacheKey}`;

    if (this.options.shouldDisableCache) {
      return;
    }

    let total = 0;
    await runCacheImprovements(
      async (cache) => {
        await cache.getNativeRef().startWriteTransaction();
      },
      () => Promise.resolve(),
    );
    try {
      report({
        type: 'cache',
        phase: 'start',
        total,
        size: this.graph.nodes.length,
      });

      let serialisedGraph = this.graph.serialize();

      // Delete an existing request graph cache, to prevent invalid states
      await this.options.cache.deleteLargeBlob(requestGraphKey);

      const serialiseAndSet = async (
        key: string,
        // $FlowFixMe serialise input is any type
        contents: any,
      ): Promise<void> => {
        if (signal?.aborted) {
          throw new Error('Serialization was aborted');
        }

        await runCacheImprovements(
          (cache) => {
            instrument(
              `RequestTracker::writeToCache::cache.put(${key})`,
              () => {
                cache.getNativeRef().putNoConfirm(key, serialize(contents));
              },
            );
            return Promise.resolve();
          },
          async () => {
            await this.options.cache.setLargeBlob(
              key,
              serialize(contents),
              signal
                ? {
                    signal: signal,
                  }
                : undefined,
            );
          },
        );

        total += 1;

        report({
          type: 'cache',
          phase: 'write',
          total,
          size: this.graph.nodes.length,
        });
      };

      let queue = new PromiseQueue({
        maxConcurrent: 32,
      });

      // Preallocating a sparse array is faster than pushing when N is high enough
      let cacheableNodes = new Array(serialisedGraph.nodes.length);
      for (let i = 0; i < serialisedGraph.nodes.length; i += 1) {
        let node = serialisedGraph.nodes[i];

        let resultCacheKey = node?.resultCacheKey;
        if (
          node?.type === REQUEST &&
          resultCacheKey != null &&
          node?.result != null
        ) {
          queue.add(() => serialiseAndSet(resultCacheKey, node.result));

          // eslint-disable-next-line no-unused-vars
          let {result: _, ...newNode} = node;
          cacheableNodes[i] = newNode;
        } else {
          cacheableNodes[i] = node;
        }
      }

      let nodeCountsPerBlob = [];

      for (
        let i = 0;
        i * this.graph.nodesPerBlob < cacheableNodes.length;
        i += 1
      ) {
        let nodesStartIndex = i * this.graph.nodesPerBlob;
        let nodesEndIndex = Math.min(
          (i + 1) * this.graph.nodesPerBlob,
          cacheableNodes.length,
        );

        nodeCountsPerBlob.push(nodesEndIndex - nodesStartIndex);

        if (!this.graph.hasCachedRequestChunk(i)) {
          // We assume the request graph nodes are immutable and won't change
          let nodesToCache = cacheableNodes.slice(
            nodesStartIndex,
            nodesEndIndex,
          );

          queue.add(() =>
            serialiseAndSet(
              getRequestGraphNodeKey(i, cacheKey),
              nodesToCache,
            ).then(() => {
              // Succeeded in writing to disk, save that we have completed this chunk
              this.graph.setCachedRequestChunk(i);
            }),
          );
        }
      }

      await queue.run();

      // Set the request graph after the queue is flushed to avoid writing an invalid state
      await serialiseAndSet(requestGraphKey, {
        ...serialisedGraph,
        nodeCountsPerBlob,
        nodes: undefined,
      });

      await runCacheImprovements(
        () =>
          serialiseAndSet(`${cacheKey}/cache_metadata`, {
            version: ATLASPACK_VERSION,
            entries: this.options.entries,
            mode: this.options.mode,
            shouldBuildLazily: this.options.shouldBuildLazily,
            watchBackend: this.options.watchBackend,
          }),
        () => Promise.resolve(),
      );

      let opts = getWatcherOptions(this.options);
      let snapshotPath = path.join(this.options.cacheDir, snapshotKey + '.txt');

      await this.options.outputFS.writeSnapshot(
        this.options.watchDir,
        snapshotPath,
        opts,
      );
    } catch (err) {
      // If we have aborted, ignore the error and continue
      if (!signal?.aborted) throw err;
    } finally {
      await runCacheImprovements(
        async (cache) => {
          await cache.getNativeRef().commitWriteTransaction();
        },
        () => Promise.resolve(),
      );
    }

    report({type: 'cache', phase: 'end', total, size: this.graph.nodes.length});
  }

  static async init({
    farm,
    options,
    rustAtlaspack,
  }: {|
    farm: WorkerFarm,
    options: AtlaspackOptions,
    rustAtlaspack?: AtlaspackV3,
  |}): Async<RequestTracker> {
    let graph = await loadRequestGraph(options);
    return new RequestTracker({farm, graph, options, rustAtlaspack});
  }
}

export function getWatcherOptions({
  watchIgnore = [],
  cacheDir,
  watchDir,
  watchBackend,
}: AtlaspackOptions): WatcherOptions {
  const vcsDirs = ['.git', '.hg'];
  const uniqueDirs = [...new Set([...watchIgnore, ...vcsDirs, cacheDir])];
  const ignore = uniqueDirs.map((dir) => path.resolve(watchDir, dir));

  return {ignore, backend: watchBackend};
}

function getCacheKey(options) {
  if (getFeatureFlag('cachePerformanceImprovements')) {
    const hash = hashString(
      `${ATLASPACK_VERSION}:${JSON.stringify(options.entries)}:${
        options.mode
      }:${options.shouldBuildLazily ? 'lazy' : 'eager'}:${
        options.watchBackend ?? ''
      }`,
    );

    return `RequestTracker/${ATLASPACK_VERSION}/${hash}`;
  }

  return hashString(
    `${ATLASPACK_VERSION}:${JSON.stringify(options.entries)}:${options.mode}:${
      options.shouldBuildLazily ? 'lazy' : 'eager'
    }:${options.watchBackend ?? ''}`,
  );
}

function getRequestGraphNodeKey(index: number, cacheKey: string) {
  if (getFeatureFlag('cachePerformanceImprovements')) {
    return `${cacheKey}/RequestGraph/nodes/${index}`;
  }

  return `requestGraph-nodes-${index}-${cacheKey}`;
}

export async function readAndDeserializeRequestGraph(
  cache: Cache,
  requestGraphKey: string,
  cacheKey: string,
): Async<{|requestGraph: RequestGraph, bufferLength: number|}> {
  let bufferLength = 0;

  const getAndDeserialize = async (key: string) => {
    if (getFeatureFlag('cachePerformanceImprovements')) {
      const buffer = await cache.getBlob(key);
      bufferLength += Buffer.byteLength(buffer);
      return deserialize(buffer);
    } else {
      const buffer = await cache.getLargeBlob(key);
      bufferLength += Buffer.byteLength(buffer);
      return deserialize(buffer);
    }
  };

  let serializedRequestGraph = await getAndDeserialize(requestGraphKey);

  let nodePromises = serializedRequestGraph.nodeCountsPerBlob.map(
    async (nodesCount, i) => {
      let nodes = await getAndDeserialize(getRequestGraphNodeKey(i, cacheKey));
      invariant.equal(
        nodes.length,
        nodesCount,
        'RequestTracker node chunk: invalid node count',
      );
      return nodes;
    },
  );

  return {
    requestGraph: RequestGraph.deserialize({
      ...serializedRequestGraph,
      nodes: (await Promise.all(nodePromises)).flat(),
    }),
    // This is used inside atlaspack query for `.inspectCache`
    bufferLength,
  };
}

async function loadRequestGraph(options): Async<RequestGraph> {
  if (options.shouldDisableCache) {
    return new RequestGraph();
  }

  let cacheKey = getCacheKey(options);
  let requestGraphKey = getFeatureFlag('cachePerformanceImprovements')
    ? `${cacheKey}/RequestGraph`
    : `requestGraph-${cacheKey}`;

  let timeout;
  const snapshotKey = getFeatureFlag('cachePerformanceImprovements')
    ? `${cacheKey}/snapshot`
    : `snapshot-${cacheKey}`;
  const snapshotPath = path.join(options.cacheDir, snapshotKey + '.txt');

  const commonMeta = {
    cacheKey,
    snapshotKey,
    cacheKeyOptions: {
      version: ATLASPACK_VERSION,
      entries: options.entries,
      mode: options.mode,
      shouldBuildLazily: options.shouldBuildLazily,
      watchBackend: options.watchBackend,
    },
  };

  logger.verbose({
    origin: '@atlaspack/core',
    message: 'Loading request graph',
    meta: {
      ...commonMeta,
    },
  });

  const hasRequestGraphInCache = getFeatureFlag('cachePerformanceImprovements')
    ? await options.cache.has(requestGraphKey)
    : await options.cache.hasLargeBlob(requestGraphKey);

  if (hasRequestGraphInCache) {
    try {
      let {requestGraph} = await readAndDeserializeRequestGraph(
        options.cache,
        requestGraphKey,
        cacheKey,
      );

      let opts = getWatcherOptions(options);

      timeout = setTimeout(() => {
        logger.warn({
          origin: '@atlaspack/core',
          message: `Retrieving file system events since last build...\nThis can take upto a minute after branch changes or npm/yarn installs.`,
        });
      }, 5000);
      let startTime = Date.now();
      let events =
        process.env.ATLASPACK_BYPASS_CACHE_INVALIDATION === 'true'
          ? []
          : await options.inputFS.getEventsSince(
              options.watchDir,
              snapshotPath,
              opts,
            );
      clearTimeout(timeout);

      logger.verbose({
        origin: '@atlaspack/core',
        message: `File system event count: ${events.length}`,
        meta: {
          ...commonMeta,
          trackableEvent: 'watcher_events_count',
          watcherEventCount: events.length,
          duration: Date.now() - startTime,
        },
      });

      if (getFeatureFlag('verboseRequestInvalidationStats')) {
        const invalidationStats = await invalidateRequestGraph(
          requestGraph,
          options,
          events,
        );

        logger.verbose({
          origin: '@atlaspack/core',
          message: 'Request track loaded from cache',
          meta: {
            ...commonMeta,
            trackableEvent: 'request_tracker_cache_key_hit',
            invalidationStats,
          },
        });
      } else {
        requestGraph.invalidateUnpredictableNodes();
        requestGraph.invalidateOnBuildNodes();
        requestGraph.invalidateEnvNodes(options.env);
        requestGraph.invalidateOptionNodes(options);
      }

      return requestGraph;
    } catch (e) {
      // Prevent logging fs events took too long warning
      clearTimeout(timeout);
      logErrorOnBailout(options, snapshotPath, e);
      // This error means respondToFSEvents timed out handling the invalidation events
      // In this case we'll return a fresh RequestGraph
      return new RequestGraph();
    }
  }

  logger.verbose({
    origin: '@atlaspack/core',
    message:
      'Cache entry for request tracker was not found, initializing a clean cache.',
    meta: {
      ...commonMeta,
      trackableEvent: 'request_tracker_cache_key_miss',
    },
  });
  return new RequestGraph();
}

/**
 * A wrapper around an invalidation type / method
 */
type InvalidationFn = {|
  key: string,
  fn: () =>
    | InvalidationDetail
    | Promise<InvalidationDetail>
    | void
    | Promise<void>,
|};

type InvalidationStats = {|
  /**
   * Total number of request graph nodes
   */
  nodeCount: number,
  /**
   * Number of requests in RequestGraph
   */
  requestCount: number,
  /**
   * Number of nodes that have been invalidated.
   */
  invalidatedCount: number,
  /**
   * Percentage of requests that have been invalidated
   */
  requestInvalidationRatio: number,
  /**
   * Percentage of nodes that have been invalidated
   */
  nodeInvalidationRatio: number,
  /**
   * Details for each invalidation type
   */
  invalidations: InvalidationFnStats[],
|};

/**
 * Details about an invalidation.
 *
 * If this is a fs events invalidation, this key will contain statistics about invalidations
 * by path.
 *
 * If this is a env or option invalidation, this key will contain the list of changed environment
 * variables or options.
 */
type InvalidationDetail = string[] | FSInvalidationStats;

/**
 * Number of invalidations for a given file-system event.
 */
type FSInvalidation = {|
  path: string,
  count: number,
|};

type FSInvalidationStats = {|
  /**
   * This list will be sorted by the number of nodes invalidated and only the top 10 will be
   * included.
   */
  biggestInvalidations: FSInvalidation[],
|};

/**
 * Information about a certain cache invalidation type.
 */
type InvalidationFnStats = {|
  /**
   * Invalidation type, one of:
   *
   * - unpredictable
   * - onBuild
   * - env
   * - option
   * - fsEvents
   */
  key: string,
  /**
   * Number of invalidated nodes coming from this invalidation type.
   */
  count: number,
  /**
   * If this is a env or option invalidation, this key will contain the list of changed values.
   *
   * If this is a fs events invalidation, this key will contain statistics about invalidations
   */
  detail: null | InvalidationDetail,
  /**
   * Time in milliseconds it took to run the invalidation.
   */
  duration: number,
|};

/**
 * Respond to unpredictable, build, environment changes, option changes and file-system events
 * invalidating RequestGraph nodes.
 *
 * Returns the count of nodes invalidated by each invalidation type.
 */
export async function invalidateRequestGraph(
  requestGraph: RequestGraph,
  options: AtlaspackOptions,
  events: Event[],
): Promise<InvalidationStats> {
  const invalidationFns: InvalidationFn[] = [
    {
      key: 'unpredictable',
      fn: () => requestGraph.invalidateUnpredictableNodes(),
    },
    {
      key: 'onBuild',
      fn: () => requestGraph.invalidateOnBuildNodes(),
    },
    {
      key: 'env',
      fn: () => requestGraph.invalidateEnvNodes(options.env),
    },
    {
      key: 'option',
      fn: () => requestGraph.invalidateOptionNodes(options),
    },
    {
      key: 'fsEvents',
      fn: () => invalidateRequestGraphFSEvents(requestGraph, options, events),
    },
  ];

  const invalidations = [];
  for (const invalidation of invalidationFns) {
    invalidations.push(await runInvalidation(requestGraph, invalidation));
  }
  const invalidatedCount = invalidations.reduce(
    (acc, invalidation) => acc + invalidation.count,
    0,
  );
  const requestCount = requestGraph.nodes.reduce(
    (acc, node) => acc + (node?.type === REQUEST ? 1 : 0),
    0,
  );
  const nodeCount = requestGraph.nodes.length;
  const nodeInvalidationRatio = invalidatedCount / nodeCount;
  const requestInvalidationRatio = invalidatedCount / requestCount;

  return {
    invalidations,
    nodeCount,
    requestCount,
    invalidatedCount,
    nodeInvalidationRatio,
    requestInvalidationRatio,
  };
}

interface InvalidateRequestGraphFSEventsInput {
  respondToFSEvents(
    events: Event[],
    options: AtlaspackOptions,
    timeout: number,
    shouldLog: boolean,
  ): Promise<{invalidationsByPath: Map<string, number>, ...}>;
}

/**
 * Invalidate the request graph based on file-system events.
 *
 * Returns statistics about the invalidations.
 */
export async function invalidateRequestGraphFSEvents(
  requestGraph: InvalidateRequestGraphFSEventsInput,
  options: AtlaspackOptions,
  events: Event[],
): Promise<FSInvalidationStats> {
  const {invalidationsByPath} = await requestGraph.respondToFSEvents(
    options.unstableFileInvalidations || events,
    options,
    10000,
    true,
  );
  const biggestInvalidations =
    getBiggestFSEventsInvalidations(invalidationsByPath);

  return {
    biggestInvalidations,
  };
}

interface RunInvalidationInput {
  getInvalidNodeCount(): number;
}

/**
 * Runs an invalidation function and reports metrics.
 */
export async function runInvalidation(
  requestGraph: RunInvalidationInput,
  invalidationFn: InvalidationFn,
): Promise<InvalidationFnStats> {
  const start = performance.now();
  const startInvalidationCount = requestGraph.getInvalidNodeCount();
  const result = await invalidationFn.fn();
  const count = requestGraph.getInvalidNodeCount() - startInvalidationCount;
  const duration = performance.now() - start;

  return {
    key: invalidationFn.key,
    count,
    detail: result ?? null,
    duration,
  };
}

function logErrorOnBailout(
  options: AtlaspackOptions,
  snapshotPath: string,
  e: Error,
): void {
  if (e.message && e.message.includes('invalid clockspec')) {
    const snapshotContents = options.inputFS.readFileSync(
      snapshotPath,
      'utf-8',
    );
    logger.warn({
      origin: '@atlaspack/core',
      message: `Error reading clockspec from snapshot, building with clean cache.`,
      meta: {
        snapshotContents: snapshotContents,
        trackableEvent: 'invalid_clockspec_error',
      },
    });
  } else {
    logger.warn({
      origin: '@atlaspack/core',
      message: `Unexpected error loading cache from disk, building with clean cache.`,
      meta: {
        errorMessage: e.message,
        errorStack: e.stack,
        trackableEvent: 'cache_load_error',
      },
    });
  }
}

export function cleanUpOrphans<N, E: number>(graph: Graph<N, E>): NodeId[] {
  if (graph.rootNodeId == null) {
    return [];
  }

  const reachableNodes = new Set();
  graph.traverse((nodeId) => {
    reachableNodes.add(nodeId);
  });

  const removedNodeIds = [];
  graph.nodes.forEach((_node, nodeId) => {
    if (!reachableNodes.has(nodeId)) {
      removedNodeIds.push(nodeId);
      graph.removeNode(nodeId);
    }
  });

  return removedNodeIds;
}

/**
 * Returns paths that invalidated the most nodes
 */
export function getBiggestFSEventsInvalidations(
  invalidationsByPath: Map<string, number>,
  limit: number = 10,
): Array<FSInvalidation> {
  const invalidations = [];
  for (const [path, count] of invalidationsByPath) {
    invalidations.push({path, count});
  }
  invalidations.sort((a, b) => b.count - a.count);

  return invalidations.slice(0, limit);
}
