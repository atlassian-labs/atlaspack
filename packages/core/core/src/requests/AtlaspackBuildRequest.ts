import type {ContentKey} from '@atlaspack/graph';
import type {Async} from '@atlaspack/types';
// @ts-expect-error - TS2614 - Module '"@atlaspack/workers"' has no exported member 'SharedReference'. Did you mean to use 'import SharedReference from "@atlaspack/workers"' instead?
import type {SharedReference} from '@atlaspack/workers';

import type {StaticRunOpts} from '../RequestTracker';
import type {Asset, AssetGroup, PackagedBundleInfo} from '../types';
import type BundleGraph from '../BundleGraph';

import createBundleGraphRequest, {
  BundleGraphResult,
} from './BundleGraphRequest';
import createWriteBundlesRequest from './WriteBundlesRequest';
import {assertSignalNotAborted} from '../utils';
import dumpGraphToGraphViz from '../dumpGraphToGraphViz';
import {bundleGraphEdgeTypes} from '../BundleGraph';
import {report} from '../ReporterRunner';
import IBundleGraph from '../public/BundleGraph';
import {NamedBundle} from '../public/Bundle';
import {assetFromValue} from '../public/Asset';

import {tracer} from '@atlaspack/profiler';
import {requestTypes} from '../RequestTracker';

type AtlaspackBuildRequestInput = {
  optionsRef: SharedReference;
  requestedAssetIds: Set<string>;
  signal?: AbortSignal;
};

export type AtlaspackBuildRequestResult = {
  bundleGraph: BundleGraph;
  bundleInfo: Map<string, PackagedBundleInfo>;
  changedAssets: Map<string, Asset>;
  assetRequests: Array<AssetGroup>;
};

type RunInput<TResult> = {
  input: AtlaspackBuildRequestInput;
} & StaticRunOpts<TResult>;

export type AtlaspackBuildRequest = {
  id: ContentKey;
  readonly type: typeof requestTypes.atlaspack_build_request;
  run: (
    arg1: RunInput<AtlaspackBuildRequestResult>,
  ) => Async<AtlaspackBuildRequestResult>;
  input: AtlaspackBuildRequestInput;
};

export default function createAtlaspackBuildRequest(
  input: AtlaspackBuildRequestInput,
): AtlaspackBuildRequest {
  return {
    type: requestTypes.atlaspack_build_request,
    id: 'atlaspack_build_request',
    run,
    input,
  };
}

// @ts-expect-error - TS7031 - Binding element 'input' implicitly has an 'any' type. | TS7031 - Binding element 'api' implicitly has an 'any' type. | TS7031 - Binding element 'options' implicitly has an 'any' type.
async function run({input, api, options}) {
  let {optionsRef, requestedAssetIds, signal} = input;

  let bundleGraphRequest = createBundleGraphRequest({
    optionsRef,
    requestedAssetIds,
    signal,
  });

  let {bundleGraph, changedAssets, assetRequests}: BundleGraphResult =
    await api.runRequest(bundleGraphRequest, {
      force: options.shouldBuildLazily && requestedAssetIds.size > 0,
    });

  // @ts-expect-error - TS2345 - Argument of type 'ContentGraph<BundleGraphNode, BundleGraphEdgeType>' is not assignable to parameter of type 'Graph<AssetGraphNode, 1> | Graph<{ assets: Set<Asset>; sourceBundles: Set<number>; bundleBehavior?: BundleBehavior | null | undefined; }, 1> | Graph<...>'.
  dumpGraphToGraphViz(bundleGraph._graph, 'BundleGraph', bundleGraphEdgeTypes);

  await report({
    type: 'buildProgress',
    phase: 'bundled',
    bundleGraph: new IBundleGraph(
      bundleGraph,
      (bundle: Bundle, bundleGraph: BundleGraph, options: AtlaspackOptions) =>
        NamedBundle.get(bundle, bundleGraph, options),
      options,
    ),
    changedAssets: new Map(
      Array.from(changedAssets).map(([id, asset]: [any, any]) => [
        id,
        assetFromValue(asset, options),
      ]),
    ),
  });

  let packagingMeasurement = tracer.createMeasurement('packaging');
  let writeBundlesRequest = createWriteBundlesRequest({
    bundleGraph,
    optionsRef,
  });

  let bundleInfo = await api.runRequest(writeBundlesRequest);
  packagingMeasurement && packagingMeasurement.end();
  assertSignalNotAborted(signal);

  return {bundleGraph, bundleInfo, changedAssets, assetRequests};
}
