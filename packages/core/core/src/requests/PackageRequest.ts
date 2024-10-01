import type {ContentKey} from '@atlaspack/graph';
import type {Async} from '@atlaspack/types';
// @ts-expect-error - TS2614 - Module '"@atlaspack/workers"' has no exported member 'SharedReference'. Did you mean to use 'import SharedReference from "@atlaspack/workers"' instead?
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

// @ts-expect-error - TS7031 - Binding element 'input' implicitly has an 'any' type. | TS7031 - Binding element 'api' implicitly has an 'any' type. | TS7031 - Binding element 'farm' implicitly has an 'any' type.
async function run({input, api, farm}) {
  let {bundleGraphReference, optionsRef, bundle, useMainThread} = input;
  let runPackage = farm.createHandle('runPackage', useMainThread);

  let start = Date.now();
  let {devDeps, invalidDevDeps} = await getDevDepRequests(api);
  let {cachePath} = nullthrows(
    // @ts-expect-error - TS2347 - Untyped function calls may not accept type arguments.
    await api.runRequest<null, ConfigAndCachePath>(
      createAtlaspackConfigRequest(),
    ),
  );

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
        // @ts-expect-error - TS2339 - Property 'type' does not exist on type 'never'.
        throw new Error(`Unknown invalidation type: ${invalidation.type}`);
    }
  }

  // @ts-expect-error - TS2540 - Cannot assign to 'time' because it is a read-only property.
  bundleInfo.time = Date.now() - start;

  api.storeResult(bundleInfo);
  return bundleInfo;
}
