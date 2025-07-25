// @flow strict-local

import type {ContentKey} from '@atlaspack/graph';
import type {Async} from '@atlaspack/types';
import type {StaticRunOpts} from '../RequestTracker';
import type {
  AssetRequestInput,
  AssetRequestResult,
  TransformationRequest,
} from '../types';
import type {ConfigAndCachePath} from './AtlaspackConfigRequest';
import type {TransformationResult} from '../Transformation';

import nullthrows from 'nullthrows';
import ThrowableDiagnostic from '@atlaspack/diagnostic';
import {hashString} from '@atlaspack/rust';
import createAtlaspackConfigRequest from './AtlaspackConfigRequest';
import {runDevDepRequest} from './DevDepRequest';
import {runConfigRequest} from './ConfigRequest';
import {fromProjectPath, fromProjectPathRelative} from '../projectPath';
import {report} from '../ReporterRunner';
import {requestTypes} from '../RequestTracker';
import type {DevDepRequestResult} from './DevDepRequest';
import {toEnvironmentId} from '../EnvironmentManager';

type RunInput<TResult> = {|
  input: AssetRequestInput,
  ...StaticRunOpts<TResult>,
|};

export type AssetRequest = {|
  id: ContentKey,
  +type: typeof requestTypes.asset_request,
  run: (RunInput<AssetRequestResult>) => Async<AssetRequestResult>,
  input: AssetRequestInput,
|};

export default function createAssetRequest(
  input: AssetRequestInput,
): AssetRequest {
  return {
    type: requestTypes.asset_request,
    id: getId(input),
    run,
    input,
  };
}

const type = 'asset_request';

function getId(input: AssetRequestInput) {
  return hashString(
    type +
      fromProjectPathRelative(input.filePath) +
      toEnvironmentId(input.env) +
      String(input.isSource) +
      String(input.sideEffects) +
      (input.code ?? '') +
      ':' +
      (input.pipeline ?? '') +
      ':' +
      (input.query ?? ''),
  );
}

async function run({input, api, farm, invalidateReason, options}) {
  report({
    type: 'buildProgress',
    phase: 'transforming',
    filePath: fromProjectPath(options.projectRoot, input.filePath),
  });

  api.invalidateOnFileUpdate(input.filePath);
  let start = Date.now();
  let {optionsRef, ...rest} = input;
  let {cachePath} = nullthrows(
    await api.runRequest<null, ConfigAndCachePath>(
      createAtlaspackConfigRequest(),
    ),
  );

  let previousDevDepRequests: Map<string, DevDepRequestResult> = new Map(
    await Promise.all(
      api
        .getSubRequests()
        .filter((req) => req.requestType === requestTypes.dev_dep_request)
        .map(async (req) => [
          req.id,
          nullthrows(await api.getRequestResult<DevDepRequestResult>(req.id)),
        ]),
    ),
  );

  let request: TransformationRequest = {
    ...rest,
    invalidateReason,
    devDeps: new Map(
      [...previousDevDepRequests.entries()]
        .filter(([id]) => api.canSkipSubrequest(id))
        .map(([, req]: [string, DevDepRequestResult]) => [
          `${req.specifier}:${fromProjectPathRelative(req.resolveFrom)}`,
          req.hash,
        ]),
    ),
    invalidDevDeps: await Promise.all(
      [...previousDevDepRequests.entries()]
        .filter(([id]) => !api.canSkipSubrequest(id))
        .flatMap(([, req]: [string, DevDepRequestResult]) => {
          return [
            {
              specifier: req.specifier,
              resolveFrom: req.resolveFrom,
            },
            ...(req.additionalInvalidations ?? []).map((i) => ({
              specifier: i.specifier,
              resolveFrom: i.resolveFrom,
            })),
          ];
        }),
    ),
  };

  let {assets, configRequests, error, invalidations, devDepRequests} =
    (await farm.createHandle(
      'runTransform',
      input.isSingleChangeRebuild,
    )({
      configCachePath: cachePath,
      optionsRef,
      request,
    }): TransformationResult);

  let time = Date.now() - start;
  if (assets) {
    for (let asset of assets) {
      asset.stats.time = time;
    }
  }

  for (let filePath of invalidations.invalidateOnFileChange) {
    api.invalidateOnFileUpdate(filePath);
    api.invalidateOnFileDelete(filePath);
  }

  for (let invalidation of invalidations.invalidateOnFileCreate) {
    api.invalidateOnFileCreate(invalidation);
  }

  for (let env of invalidations.invalidateOnEnvChange) {
    api.invalidateOnEnvChange(env);
  }

  for (let option of invalidations.invalidateOnOptionChange) {
    api.invalidateOnOptionChange(option);
  }

  if (invalidations.invalidateOnStartup) {
    api.invalidateOnStartup();
  }

  if (invalidations.invalidateOnBuild) {
    api.invalidateOnBuild();
  }

  for (let devDepRequest of devDepRequests) {
    await runDevDepRequest(api, devDepRequest);
  }

  for (let configRequest of configRequests) {
    await runConfigRequest(api, configRequest);
  }

  if (error != null) {
    throw new ThrowableDiagnostic({diagnostic: error});
  } else {
    return nullthrows(assets);
  }
}
