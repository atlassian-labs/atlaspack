// eslint-disable-next-line flowtype/no-types-missing-file-annotation
import type {
  InitialAtlaspackOptions,
  BuildEvent,
  BuildSuccessEvent,
  AsyncSubscription,
} from '@atlaspack/types';
// eslint-disable-next-line flowtype/no-types-missing-file-annotation
import type {FarmOptions} from '@atlaspack/workers';
// eslint-disable-next-line flowtype/no-types-missing-file-annotation
import type WorkerFarm from '@atlaspack/workers';

// eslint-disable-next-line flowtype/no-types-missing-file-annotation
export declare const ATLASPACK_VERSION: string;

// eslint-disable-next-line flowtype/no-types-missing-file-annotation
export class Atlaspack {
  constructor(options: InitialAtlaspackOptions);
  run(): Promise<BuildSuccessEvent>;
  watch(
    cb?: (err: Error | null | undefined, buildEvent?: BuildEvent) => unknown,
  ): Promise<AsyncSubscription>;
}

// eslint-disable-next-line flowtype/no-types-missing-file-annotation
export class Parcel {
  constructor(options: InitialAtlaspackOptions);
  run(): Promise<BuildSuccessEvent>;
  watch(
    cb?: (err: Error | null | undefined, buildEvent?: BuildEvent) => unknown,
  ): Promise<AsyncSubscription>;
}

// eslint-disable-next-line flowtype/no-types-missing-file-annotation
export declare function createWorkerFarm(
  options?: Partial<FarmOptions>,
): WorkerFarm;

// eslint-disable-next-line flowtype/no-types-missing-file-annotation
export default Atlaspack;
