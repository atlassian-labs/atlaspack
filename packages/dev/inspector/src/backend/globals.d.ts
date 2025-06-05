declare module '@atlaspack/feature-flags' {
  export type FeatureFlags = Record<string, boolean | string | number>;
  export const DEFAULT_FEATURE_FLAGS: FeatureFlags;
  export function setFeatureFlags(flags: FeatureFlags): void;
  export function getFeatureFlag(flagName: string): boolean;
  export function getFeatureFlagValue(
    flagName: string,
  ): boolean | string | number;
}

declare module '@atlaspack/graph' {
  export const ALL_EDGE_TYPES: number;

  export class Graph<Node> {
    nodes: Node[];
    getNode: (id: string) => Node;
    getNodeIdByContentKey: (key: string) => string;
    getNodeIdsConnectedFrom: (id: string, types: number | number[]) => string[];
    dfs: (options: any) => void;
    traverse: (options: any, startId: string, types: number | number[]) => void;
    rootNodeId: string;
  }
}

declare module '@atlaspack/core/lib/types.js' {
  export interface Node {
    type: string;
    id: string;
    value: any;
  }

  export interface CacheResult {
    assetGraph?: any;
    bundleGraph?: any;
  }
}

declare module '@atlaspack/core/lib/AssetGraph.js' {
  import type {Node} from '@atlaspack/core/lib/types.js';

  export default class AssetGraph {
    getNode(id: string): Node;
    getNodeIdByContentKey(key: string): string;
    deserialize(value: any): AssetGraph;
  }
}

declare module '@atlaspack/core/lib/BundleGraph.js' {
  import type {Node} from '@atlaspack/core/lib/types.js';

  export class BundleGraph {
    _graph: {
      nodes: Node[];
      getNode: (id: string) => Node;
      getNodeIdByContentKey: (key: string) => string;
      getNodeIdsConnectedFrom: (
        id: string,
        types: number | number[],
      ) => string[];
      dfs: (options: any) => void;
      traverse: (
        options: any,
        startId: string,
        types: number | number[],
      ) => void;
      rootNodeId: string;
    };
    traverseBundle(bundle: Node, options: any): void;
    getChildBundles(bundle: Node): Node[];
    static deserialize(value: any): BundleGraph;
  }

  export default BundleGraph;
}

declare module '@atlaspack/core/lib/RequestTracker.js' {
  export const requestTypes: Record<string, number>;

  export function readAndDeserializeRequestGraph(
    cache: LMDBLiteCache,
    requestGraphBlob: string,
    requestGraphKey: string,
  ): Promise<{requestGraph: RequestGraph}>;

  export interface RequestGraphNode {
    id: string;
    type: number;
    requestType: number;
    result?: {
      bundleId?: string;
      filePath?: string;
    };
    resultCacheKey?: string;
  }

  export class RequestTracker {
    graph: {
      nodes: RequestGraphNode[];
    };
  }

  export const requestTypes: {
    atlaspack_build_request: number;
    bundle_graph_request: number;
    asset_graph_request: number;
    entry_request: number;
    target_request: number;
    atlaspack_config_request: number;
    path_request: number;
    dev_dep_request: number;
    asset_request: number;
    config_request: number;
    write_bundles_request: number;
    package_request: number;
    write_bundle_request: number;
    validation_request: number;
  };

  export default RequestTracker;
}

declare module '@atlaspack/query' {
  import type {BundleGraph, RequestTracker, AssetGraph} from '@atlaspack/core';

  export function loadGraphs(cacheDir: string): Promise<{
    requestTracker: RequestTracker;
    bundleGraph: BundleGraph;
    assetGraph: AssetGraph;
  }>;
}
