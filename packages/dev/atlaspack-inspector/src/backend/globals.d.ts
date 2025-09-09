declare module '@atlaspack/query' {
  import type {BundleGraph, RequestTracker, AssetGraph} from '@atlaspack/core';

  export function loadGraphs(cacheDir: string): Promise<{
    requestTracker: RequestTracker;
    bundleGraph: BundleGraph;
    assetGraph: AssetGraph;
  }>;
}
