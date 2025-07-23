import type {
  InitialAtlaspackOptions,
  BuildEvent,
  BuildSuccessEvent,
  AsyncSubscription,
} from '@atlaspack/types';

import type {FarmOptions} from '@atlaspack/workers';

import type WorkerFarm from '@atlaspack/workers';

export declare const ATLASPACK_VERSION: string;

export type Transferable = any;

export class NapiWorkerPool {
  constructor({workerCount}: {workerCount: number});

  workerCount(): number;

  getWorkers(): Promise<Array<Transferable>>;

  shutdown(): void;
}

export class Atlaspack {
  constructor(options: InitialAtlaspackOptions);
  run(): Promise<BuildSuccessEvent>;
  watch(
    cb?: (err: Error | null | undefined, buildEvent?: BuildEvent) => unknown,
  ): Promise<AsyncSubscription>;
}

export class Parcel {
  constructor(options: InitialAtlaspackOptions);
  run(): Promise<BuildSuccessEvent>;
  watch(
    cb?: (err: Error | null | undefined, buildEvent?: BuildEvent) => unknown,
  ): Promise<AsyncSubscription>;
}

export declare function createWorkerFarm(
  options?: Partial<FarmOptions>,
): WorkerFarm;

export default Atlaspack;
