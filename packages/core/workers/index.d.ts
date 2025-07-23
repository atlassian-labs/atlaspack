// eslint-disable-next-line import/no-extraneous-dependencies
import type {FilePath} from '@atlaspack/types';

type BackendType = 'process' | 'threads';

export type FarmOptions = {
  maxConcurrentWorkers: number;
  maxConcurrentCallsPerWorker: number;
  forcedKillTime: number;
  useLocalWorker: boolean;
  warmWorkers: boolean;
  workerPath?: FilePath;
  backend: BackendType;
  shouldPatchConsole?: boolean;
  shouldTrace?: boolean;
};

export type SharedReference = number;

export type LocationCallRequest = {
  args: ReadonlyArray<unknown>;
  location: string;
  method?: string;
};

export type HandleCallRequest = {
  args: ReadonlyArray<unknown>;
  handle: number;
};

export type CallRequest = LocationCallRequest | HandleCallRequest;

declare class WorkerFarm {
  constructor(options: FarmOptions);

  end(): Promise<void>;

  createReverseHandle(fn: (...args: Array<any>) => unknown): Handle;

  workerApi: {
    callChild: (
      childId: number,
      request: HandleCallRequest,
    ) => Promise<unknown>;
    callMaster: (
      request: CallRequest,
      awaitResponse?: boolean | null | undefined,
    ) => Promise<unknown>;
    createReverseHandle: (fn: HandleFunction) => Handle;
    getSharedReference: (ref: SharedReference) => unknown;
    resolveSharedReference: (value: unknown) => undefined | SharedReference;
    runHandle: (handle: Handle, args: Array<any>) => Promise<unknown>;
  };

  static isWorker(): boolean;

  static getWorkerApi(): {
    callMaster: (
      request: CallRequest,
      awaitResponse?: boolean | null | undefined,
    ) => Promise<unknown>;
    createReverseHandle: (fn: (...args: Array<any>) => unknown) => Handle;
    getSharedReference: (ref: SharedReference) => unknown;
    resolveSharedReference: (value: unknown) => undefined | SharedReference;
    runHandle: (handle: Handle, args: Array<any>) => Promise<unknown>;
  };
}

export declare class Handle {
  id: number;
  childId: number | null | undefined;
  fn: HandleFunction | null | undefined;

  constructor(opts: HandleOpts);

  dispose(): void;

  serialize(): {
    childId: number | null | undefined;
    id: number;
  };

  static deserialize(opts: HandleOpts): Handle;
}

export default WorkerFarm;
