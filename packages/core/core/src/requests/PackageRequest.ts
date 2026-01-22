import type {ContentKey} from '@atlaspack/graph';
import type {Async} from '@atlaspack/types';
import type {SharedReference} from '@atlaspack/workers';

import type {StaticRunOpts} from '../RequestTracker';
import {requestTypes} from '../RequestTracker';
import type {Bundle} from '../types';
import type BundleGraph from '../BundleGraph';
import type {BundleInfo, RunPackagerRunnerResult} from '../PackagerRunner';
import type {ConfigAndCachePath} from './AtlaspackConfigRequest';

import nullthrows from 'nullthrows';
import {runConfigRequest} from './ConfigRequest';
import {getDevDepRequests, runDevDepRequest} from './DevDepRequest';
import createAtlaspackConfigRequest from './AtlaspackConfigRequest';
import {fromEnvironmentId} from '../EnvironmentManager';
import {getFeatureFlag} from '@atlaspack/feature-flags';

type PackageRequestInput = {
  bundleGraph: BundleGraph;
  bundle: Bundle;
  bundleGraphReference: SharedReference;
  optionsRef: SharedReference;
  useMainThread?: boolean;
};

export type PackageRequestResult = BundleInfo;

type RunInput<TResult> = {
  input: PackageRequestInput;
} & StaticRunOpts<TResult>;

export type PackageRequest = {
  id: ContentKey;
  readonly type: typeof requestTypes.package_request;
  run: (arg1: RunInput<BundleInfo>) => Async<BundleInfo>;
  input: PackageRequestInput;
};

export function createPackageRequest(
  input: PackageRequestInput,
): PackageRequest {
  return {
    type: requestTypes.package_request,
    id: input.bundleGraph.getHash(input.bundle),
    run,
    input,
  };
}

async function run({input, api, farm, rustAtlaspack}: RunInput<BundleInfo>) {
  let {bundleGraphReference, optionsRef, bundle, useMainThread} = input;

  let runPackage = farm.createHandle('runPackage', useMainThread);

  let start = Date.now();
  let {devDeps, invalidDevDeps} = await getDevDepRequests(api);
  let {cachePath} = nullthrows(
    await api.runRequest<null, ConfigAndCachePath>(
      createAtlaspackConfigRequest(),
    ),
  );

  if (
    getFeatureFlag('nativePackager') &&
    getFeatureFlag('nativePackagerSSRDev') &&
    rustAtlaspack &&
    fromEnvironmentId(bundle.env).context === 'tesseract' &&
    bundle.type === 'js'
  ) {
    // Once this actually does something, the code below will be in an `else` block (i.e. we'll only run one or the other)
    await rustAtlaspack.package();
  }

  let {devDepRequests, configRequests, bundleInfo, invalidations} =
    (await runPackage({
      bundle,
      bundleGraphReference,
      optionsRef,
      configCachePath: cachePath,
      previousDevDeps: devDeps,
      invalidDevDeps,
      previousInvalidations: api.getInvalidations(),
    })) as RunPackagerRunnerResult;

  for (let devDepRequest of devDepRequests) {
    await runDevDepRequest(api, devDepRequest);
  }

  for (let configRequest of configRequests) {
    await runConfigRequest(api, configRequest);
  }

  for (let invalidation of invalidations) {
    switch (invalidation.type) {
      case 'file':
        api.invalidateOnFileUpdate(invalidation.filePath);
        api.invalidateOnFileDelete(invalidation.filePath);
        break;
      case 'env':
        api.invalidateOnEnvChange(invalidation.key);
        break;
      case 'option':
        api.invalidateOnOptionChange(invalidation.key);
        break;
      default:
        // @ts-expect-error TS2339
        throw new Error(`Unknown invalidation type: ${invalidation.type}`);
    }
  }

  // @ts-expect-error TS2540
  bundleInfo.time = Date.now() - start;

  api.storeResult(bundleInfo);
  return bundleInfo;
}
