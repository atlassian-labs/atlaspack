import type {ContentKey} from '@atlaspack/graph';
import type {Async} from '@atlaspack/types';
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
import {getFeatureFlag} from '@atlaspack/feature-flags';
import {fromEnvironmentId} from '../EnvironmentManager';

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
  scopeHoistingStats?: {
    totalAssets: number;
    wrappedAssets: number;
  };
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

async function run({
  input,
  api,
  options,
  rustAtlaspack,
}: RunInput<AtlaspackBuildRequestResult>) {
  let {optionsRef, requestedAssetIds, signal} = input;

  let bundleGraphRequest = createBundleGraphRequest({
    optionsRef,
    requestedAssetIds,
    signal,
  });

  let {bundleGraph, changedAssets, assetRequests}: BundleGraphResult =
    await api.runRequest(bundleGraphRequest, {
      force:
        Boolean(rustAtlaspack) ||
        (options.shouldBuildLazily && requestedAssetIds.size > 0),
    });

  if (
    getFeatureFlag('nativePackager') &&
    getFeatureFlag('nativePackagerSSRDev') &&
    rustAtlaspack
  ) {
    let hasTesseractTarget = false;
    bundleGraph.traverseBundles((bundle, ctx, actions) => {
      if (fromEnvironmentId(bundle.env).context === 'tesseract') {
        hasTesseractTarget = true;
        actions.stop();
      }
    });
    if (hasTesseractTarget) {
      // eslint-disable-next-line no-console
      console.log('I WOULD SEND THE BUNDLE GRAPH TO RUST HERE');
    }
  }

  // @ts-expect-error TS2345
  dumpGraphToGraphViz(bundleGraph._graph, 'BundleGraph', bundleGraphEdgeTypes);

  await report({
    type: 'buildProgress',
    phase: 'bundled',
    bundleGraph: new IBundleGraph(
      bundleGraph,
      // @ts-expect-error TS2304
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

  let {bundleInfo, scopeHoistingStats} =
    await api.runRequest(writeBundlesRequest);
  packagingMeasurement && packagingMeasurement.end();
  assertSignalNotAborted(signal);

  return {
    bundleGraph,
    bundleInfo,
    changedAssets,
    assetRequests,
    scopeHoistingStats,
  };
}
