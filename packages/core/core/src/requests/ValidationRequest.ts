import type {Async} from '@atlaspack/types';
// @ts-expect-error - TS2614 - Module '"@atlaspack/workers"' has no exported member 'SharedReference'. Did you mean to use 'import SharedReference from "@atlaspack/workers"' instead?
import type {SharedReference} from '@atlaspack/workers';
import type {StaticRunOpts} from '../RequestTracker';
import type {AssetGroup} from '../types';
import type {ConfigAndCachePath} from './AtlaspackConfigRequest';

import nullthrows from 'nullthrows';
import AtlaspackConfig from '../AtlaspackConfig';
import {report} from '../ReporterRunner';
import Validation from '../Validation';
import createAtlaspackConfigRequest from './AtlaspackConfigRequest';
import {requestTypes} from '../RequestTracker';

type ValidationRequest = {
  id: string;
  readonly type: typeof requestTypes.validation_request;
  run: (arg1: RunOpts<undefined>) => Async<void>;
  input: ValidationRequestInput;
};

type RunOpts<TResult> = {
  input: ValidationRequestInput;
} & StaticRunOpts<TResult>;

type ValidationRequestInput = {
  assetRequests: Array<AssetGroup>;
  optionsRef: SharedReference;
};

export default function createValidationRequest(
  input: ValidationRequestInput,
): ValidationRequest {
  return {
    id: 'validation',
    type: requestTypes.validation_request,
    run: async ({input: {assetRequests, optionsRef}, api, options, farm}) => {
      let {config: processedConfig, cachePath} = nullthrows(
        await api.runRequest<null, ConfigAndCachePath>(
          createAtlaspackConfigRequest(),
        ),
      );

      let config = new AtlaspackConfig(processedConfig, options);
      let trackedRequestsDesc = assetRequests.filter((request) => {
        return config.getValidatorNames(request.filePath).length > 0;
      });

      // Schedule validations on workers for all plugins that implement the one-asset-at-a-time "validate" method.
      let promises = trackedRequestsDesc.map(
        async (request) =>
          // @ts-expect-error - TS2339 - Property 'createHandle' does not exist on type 'WorkerFarm'.
          (await farm.createHandle('runValidate'))({
            requests: [request],
            optionsRef: optionsRef,
            configCachePath: cachePath,
          }) as undefined,
      );

      // Skip sending validation requests if no validators were configured
      if (trackedRequestsDesc.length === 0) {
        return;
      }

      // Schedule validations on the main thread for all validation plugins that implement "validateAll".
      promises.push(
        // @ts-expect-error - TS2345 - Argument of type 'Promise<void>' is not assignable to parameter of type 'Promise<undefined>'.
        new Validation({
          requests: trackedRequestsDesc,
          options,
          config,
          // @ts-expect-error - TS2322 - Type '(event: ReporterEvent) => Promise<void>' is not assignable to type 'ReportFn'.
          report,
          dedicatedThread: true,
        }).run(),
      );
      await Promise.all(promises);
    },
    input,
  };
}
