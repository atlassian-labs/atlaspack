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

export interface RequestTracker {
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
