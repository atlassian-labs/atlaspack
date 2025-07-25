// eslint-disable-next-line import/no-extraneous-dependencies
import type {FilePath} from '@atlaspack/types';
import type EventEmitter from 'events';

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

export class Bus extends EventEmitter {
  emit(event: string, ...args: Array<any>): boolean;
}

export const bus: Bus;

export declare class WorkerFarm {
  ending: boolean;
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
  constructor(options: Partial<FarmOptions>);
  createSharedReference(
    value: unknown,
    isCacheable?: boolean,
  ): {
    ref: SharedReference;
    dispose(): Promise<unknown>;
  };
  startProfile(): Promise<void>;
  endProfile(): Promise<void>;
  takeHeapSnapshot(): Promise<void>;
  createHandle(method: string, useMainThread?: boolean): HandleFunction;
  createReverseHandle(fn: HandleFunction): Handle;
  callAllWorkers(method: string, args: Array<any>): Promise<void>;
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
  end(): Promise<void>;
  static isWorker(): boolean;
}

export default WorkerFarm;

export type SharedReference = number;

export type WorkerApi = {
  callMaster(
    arg1: CallRequest,
    arg2?: boolean | null | undefined,
  ): Promise<unknown>;
  createReverseHandle(fn: HandleFunction): Handle;
  getSharedReference(ref: SharedReference): unknown;
  resolveSharedReference(value: unknown): SharedReference | null | undefined;
  callChild?: (childId: number, request: HandleCallRequest) => Promise<unknown>;
};

export type HandleFunction = (...args: Array<any>) => any;

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

type HandleOpts = {
  fn?: HandleFunction;
  childId?: number | null | undefined;
  id?: number;
};

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
