import type {
  InitialParcelOptions,
  BuildEvent,
  BuildSuccessEvent,
  AsyncSubscription,
} from '@atlaspack/types';
import type {FarmOptions} from '@atlaspack/workers';
import type WorkerFarm from '@atlaspack/workers';

export class Parcel {
  constructor(options: InitialParcelOptions);
  run(): Promise<BuildSuccessEvent>;
  watch(
    cb?: (err: Error | null | undefined, buildEvent?: BuildEvent) => unknown,
  ): Promise<AsyncSubscription>;
}

export declare function createWorkerFarm(
  options?: Partial<FarmOptions>,
): WorkerFarm;

export default Parcel;
