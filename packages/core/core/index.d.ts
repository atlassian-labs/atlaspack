import type {
  InitialAtlaspackOptions,
  BuildEvent,
  BuildSuccessEvent,
  AsyncSubscription,
} from '@atlaspack/types';
import type {Diagnostic} from '@atlaspack/diagnostic';
import type ThrowableDiagnostic from '@atlaspack/diagnostic';

import type {FarmOptions} from '@atlaspack/workers';

import type WorkerFarm from '@atlaspack/workers';

export declare const ATLASPACK_VERSION: string;

export class Atlaspack {
  constructor(options: InitialAtlaspackOptions);
  run(): Promise<BuildSuccessEvent>;
  watch(
    cb?: (err: Error | null | undefined, buildEvent?: BuildEvent) => unknown,
  ): Promise<AsyncSubscription>;
  _init(): Promise<void>;
}

export class Parcel {
  constructor(options: InitialAtlaspackOptions);
  run(): Promise<BuildSuccessEvent>;
  watch(
    cb?: (err: Error | null | undefined, buildEvent?: BuildEvent) => unknown,
  ): Promise<AsyncSubscription>;
  _init(): Promise<void>;
}

export declare function createWorkerFarm(
  options?: Partial<FarmOptions>,
): WorkerFarm;

export default Atlaspack;

export type NapiWorkerPoolOptions = {
  workerCount?: number;
};

export type Transferable = any;

export declare class NapiWorkerPool {
  constructor(options?: NapiWorkerPoolOptions);
  workerCount(): number;
  getWorkers(): Promise<Array<Transferable>>;
  shutdown(): void;
}

export declare class BuildError extends ThrowableDiagnostic {
  constructor(diagnostic: Array<Diagnostic> | Diagnostic);
}

export const INTERNAL_TRANSFORM: symbol;
export const INTERNAL_RESOLVE: symbol;
export let WORKER_PATH: string;
