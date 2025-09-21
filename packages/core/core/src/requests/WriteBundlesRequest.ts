import type {ContentKey} from '@atlaspack/graph';
import type {Async} from '@atlaspack/types';
import {getFeatureFlag} from '@atlaspack/feature-flags';
import type {SharedReference} from '@atlaspack/workers';
import type {StaticRunOpts} from '../RequestTracker';
import {requestTypes} from '../RequestTracker';
import {
  BundleBehavior,
  type PackagedBundleInfo,
  type Bundle,
  type AtlaspackOptions,
} from '../types';
import type BundleGraph from '../BundleGraph';
import type {BundleInfo} from '../PackagerRunner';
import {report} from '../ReporterRunner';

import {HASH_REF_PREFIX} from '../constants';
import {joinProjectPath} from '../projectPath';
import nullthrows from 'nullthrows';
import {hashString} from '@atlaspack/rust';
import {createPackageRequest} from './PackageRequest';
import createWriteBundleRequest from './WriteBundleRequest';

type WriteBundlesRequestInput = {
  bundleGraph: BundleGraph;
  optionsRef: SharedReference;
};

export type WriteBundlesRequestResult = {
  bundleInfoMap: Map<string, PackagedBundleInfo>;
  scopeHoistingStats?: {
    totalAssets: number;
    wrappedAssets: number;
  };
};

type RunInput<TResult> = {
  input: WriteBundlesRequestInput;
} & StaticRunOpts<TResult>;

export type WriteBundlesRequest = {
  id: ContentKey;
  readonly type: typeof requestTypes.write_bundles_request;
  run: (
    arg1: RunInput<WriteBundlesRequestResult>,
  ) => Async<WriteBundlesRequestResult>;
  input: WriteBundlesRequestInput;
};

function reportPackagingProgress(
  completeBundles: number,
  totalBundles: number,
) {
  if (!getFeatureFlag('cliProgressReportingImprovements')) {
    return;
  }

  report({
    type: 'buildProgress',
    phase: 'packagingAndOptimizing',
    totalBundles,
    completeBundles,
  });
}

/**
 * Packages, optimizes, and writes all bundles to the dist directory.
 */
export default function createWriteBundlesRequest(
  input: WriteBundlesRequestInput,
): WriteBundlesRequest {
  return {
    type: requestTypes.write_bundles_request,
    id: 'write_bundles:' + input.bundleGraph.getBundleGraphHash(),
    run,
    input,
  };
}

async function run({
  input,
  api,
  farm,
  options,
}: RunInput<WriteBundlesRequestResult>) {
  let {bundleGraph, optionsRef} = input;
  let {ref, dispose} = await farm.createSharedReference(bundleGraph);

  api.invalidateOnOptionChange('shouldContentHash');

  let res = new Map();
  let bundleInfoMap: {
    [key: string]: BundleInfo;
  } = {};
  let writeEarlyPromises: Record<string, Promise<PackagedBundleInfo>> = {};
  let hashRefToNameHash = new Map();

  // Include inline bundles so that non-inline bundles referenced from inline bundles are written to
  // separate files. This ensures that source maps are written and work.
  const allBundles = bundleGraph.getBundles({
    includeInline: getFeatureFlag('inlineBundlesSourceMapFixes'),
  });
  const bundles = allBundles
    .filter(
      (bundle) =>
        bundle.bundleBehavior !== BundleBehavior.inline &&
        bundle.bundleBehavior !== BundleBehavior.inlineIsolated,
    )
    .filter((bundle) => {
      // Do not package and write placeholder bundles to disk. We just
      // need to update the name so other bundles can reference it.
      if (bundle.isPlaceholder) {
        let hash = bundle.id.slice(-8);
        hashRefToNameHash.set(bundle.hashReference, hash);
        let name = nullthrows(
          bundle.name,
          `Expected ${bundle.type} bundle to have a name`,
        ).replace(bundle.hashReference, hash);
        res.set(bundle.id, {
          filePath: joinProjectPath(bundle.target.distDir, name),
          bundleId: bundle.id,
          type: bundle.type, // FIXME: this is wrong if the packager changes the type...
          stats: {
            time: 0,
            size: 0,
          },
        });
        return false;
      }

      return true;
    });

  let cachedBundles = new Set(
    bundles.filter((b) => api.canSkipSubrequest(bundleGraph.getHash(b))),
  );

  // Package on the main thread if there is only one bundle to package.
  // This avoids the cost of serializing the bundle graph for single file change builds.
  let useMainThread =
    bundles.length === 1 || bundles.length - cachedBundles.size <= 1;

  try {
    let completeBundles = cachedBundles.size;
    reportPackagingProgress(completeBundles, bundles.length);

    await Promise.all(
      bundles.map(async (bundle) => {
        let request = createPackageRequest({
          bundle,
          bundleGraph,
          bundleGraphReference: ref,
          optionsRef,
          useMainThread,
        });

        let info = await api.runRequest(request);

        if (!cachedBundles.has(bundle)) {
          completeBundles++;
          reportPackagingProgress(completeBundles, bundles.length);
        }

        if (!useMainThread) {
          // Force a refresh of the cache to avoid a race condition
          // between threaded reads and writes that can result in an LMDB cache miss:
          //   1. The main thread has read some value from cache, necessitating a read transaction.
          //   2. Concurrently, Thread A finishes a packaging request.
          //   3. Subsequently, the main thread is tasked with this request, but fails because the read transaction is stale.
          // This only occurs if the reading thread has a transaction that was created before the writing thread committed,
          // and the transaction is still live when the reading thread attempts to get the written value.
          // See https://github.com/parcel-bundler/parcel/issues/9121
          options.cache.refresh();
        }

        bundleInfoMap[bundle.id] = info;
        if (!info.hashReferences.length) {
          hashRefToNameHash.set(
            bundle.hashReference,
            options.shouldContentHash
              ? info.hash.slice(-8)
              : bundle.id.slice(-8),
          );
          let writeBundleRequest = createWriteBundleRequest({
            bundle,
            info,
            hashRefToNameHash,
            bundleGraph,
          });
          let promise = api.runRequest(writeBundleRequest);
          // If the promise rejects before we await it (below), we don't want to crash the build.
          promise.catch(() => {});
          writeEarlyPromises[bundle.id] = promise;
        }
      }),
    );
    assignComplexNameHashes(hashRefToNameHash, bundles, bundleInfoMap, options);
    await Promise.all(
      bundles.map((bundle) => {
        let promise =
          writeEarlyPromises[bundle.id] ??
          api.runRequest(
            createWriteBundleRequest({
              bundle,
              info: bundleInfoMap[bundle.id],
              hashRefToNameHash,
              bundleGraph,
            }),
          );

        return promise.then((r) => res.set(bundle.id, r));
      }),
    );

    // Aggregate scope hoisting stats from all bundles
    let aggregatedScopeHoistingStats = {
      totalAssets: 0,
      wrappedAssets: 0,
    };
    let hasScopeHoistingStats = false;

    for (let bundle of bundles) {
      let bundleInfo = bundleInfoMap[bundle.id];
      if (bundleInfo?.scopeHoistingStats) {
        hasScopeHoistingStats = true;
        aggregatedScopeHoistingStats.totalAssets += bundleInfo.scopeHoistingStats.totalAssets;
        aggregatedScopeHoistingStats.wrappedAssets += bundleInfo.scopeHoistingStats.wrappedAssets;
      }
    }

    let result = {
      bundleInfoMap: res,
      scopeHoistingStats: hasScopeHoistingStats ? aggregatedScopeHoistingStats : undefined,
    };

    api.storeResult(result);
    return result;
  } finally {
    await dispose();
  }
}

function assignComplexNameHashes(
  hashRefToNameHash: Map<string, string>,
  bundles: Array<Bundle>,
  bundleInfoMap: {
    [key: string]: BundleInfo;
  },
  options: AtlaspackOptions,
) {
  for (let bundle of bundles) {
    if (hashRefToNameHash.get(bundle.hashReference) != null) {
      continue;
    }
    hashRefToNameHash.set(
      bundle.hashReference,
      options.shouldContentHash
        ? hashString(
            [...getBundlesIncludedInHash(bundle.id, bundleInfoMap)]
              .map((bundleId) => bundleInfoMap[bundleId].hash)
              .join(':'),
          ).slice(-8)
        : bundle.id.slice(-8),
    );
  }
}

function getBundlesIncludedInHash(
  bundleId: ContentKey | string,
  bundleInfoMap: {
    [key: string]: BundleInfo;
  },
  included = new Set<string>(),
) {
  included.add(bundleId);
  for (let hashRef of bundleInfoMap[bundleId]?.hashReferences ?? []) {
    let referencedId = getIdFromHashRef(hashRef);
    if (!included.has(referencedId)) {
      getBundlesIncludedInHash(referencedId, bundleInfoMap, included);
    }
  }

  return included;
}

function getIdFromHashRef(hashRef: string) {
  return hashRef.slice(HASH_REF_PREFIX.length);
}
