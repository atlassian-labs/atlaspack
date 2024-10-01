import type {ErrorWithCode, FilePath} from '@atlaspack/types-internal';
import type {
  CallRequest,
  HandleCallRequest,
  WorkerRequest,
  WorkerDataResponse,
  WorkerErrorResponse,
  BackendType,
} from './types';
import type {HandleFunction} from './Handle';

import * as coreWorker from './core-worker';
import * as bus from './bus';
import invariant from 'assert';
import nullthrows from 'nullthrows';
import EventEmitter from 'events';
import {
  deserialize,
  prepareForSerialization,
  restoreDeserializedObject,
  serialize,
} from '@atlaspack/core';
import ThrowableDiagnostic, {anyToDiagnostic, md} from '@atlaspack/diagnostic';
import Worker, {WorkerCall} from './Worker';
import cpuCount from './cpuCount';
import Handle from './Handle';
import {child} from './childState';
import {detectBackend} from './backend';
import {SamplingProfiler, Trace} from '@atlaspack/profiler';
import fs from 'fs';
import logger from '@atlaspack/logger';

let referenceId = 1;

export type SharedReference = number;

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

type WorkerModule = {
  readonly [key: string]: (...args: Array<unknown>) => Promise<unknown>;
};

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

export {Handle};

const DEFAULT_MAX_CONCURRENT_CALLS: number = 30;

/**
 * workerPath should always be defined inside farmOptions
 */

export default class WorkerFarm extends EventEmitter {
  callQueue: Array<WorkerCall> = [];
  ending: boolean = false;
  localWorker: WorkerModule;
  localWorkerInit: Promise<undefined> | null | undefined;
  options: FarmOptions;
  run: HandleFunction;
  warmWorkers: number = 0;
  readyWorkers: number = 0;
  workers: Map<number, Worker> = new Map();
  handles: Map<number, Handle> = new Map();
  sharedReferences: Map<SharedReference, unknown> = new Map();
  sharedReferencesByValue: Map<unknown, SharedReference> = new Map();
  serializedSharedReferences: Map<
    SharedReference,
    ArrayBuffer | null | undefined
  > = new Map();
  profiler: SamplingProfiler | null | undefined;

  constructor(farmOptions: Partial<FarmOptions> = {}) {
    super();
    this.options = {
      maxConcurrentWorkers: WorkerFarm.getNumWorkers(),
      maxConcurrentCallsPerWorker: WorkerFarm.getConcurrentCallsPerWorker(
        farmOptions.shouldTrace ? 1 : DEFAULT_MAX_CONCURRENT_CALLS,
      ),
      forcedKillTime: 500,
      warmWorkers: false,
      useLocalWorker: true, // TODO: setting this to false makes some tests fail, figure out why
      backend: detectBackend(),
      ...farmOptions,
    };

    if (!this.options.workerPath) {
      throw new Error('Please provide a worker path!');
    }

    if (process.browser) {
      if (this.options.workerPath === '@atlaspack/core/src/worker.js') {
        this.localWorker = coreWorker;
      } else {
        throw new Error(
          'No dynamic require possible: ' + this.options.workerPath,
        );
      }
    } else {
      this.localWorker = require(this.options.workerPath);
    }

    this.localWorkerInit =
      this.localWorker.childInit != null ? this.localWorker.childInit() : null;

    this.run = this.createHandle('run');

    // Worker thread stdout is by default piped into the process stdout, if there are enough worker
    // threads to exceed the default listener limit, then anything else piping into stdout will trigger
    // the `MaxListenersExceededWarning`, so we should ensure the max listeners is at least equal to the
    // number of workers + 1 for the main thread.
    //
    // Note this can't be fixed easily where other things pipe into stdout -  even after starting > 10 worker
    // threads `process.stdout.getMaxListeners()` will still return 10, however adding another pipe into `stdout`
    // will give the warning with `<worker count + 1>` as the number of listeners.
    process.stdout?.setMaxListeners(
      Math.max(
        process.stdout.getMaxListeners(),
        WorkerFarm.getNumWorkers() + 1,
      ),
    );

    this.startMaxWorkers();
  }

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
  } = {
    callMaster: async (
      request: CallRequest,
      awaitResponse: boolean | null = true,
    ): Promise<unknown> => {
      let result = await this.processRequest({
        ...request,
        awaitResponse,
      });
      return deserialize(serialize(result));
    },
    createReverseHandle: (fn: HandleFunction): Handle =>
      this.createReverseHandle(fn),
    callChild: (
      childId: number,
      request: HandleCallRequest,
    ): Promise<unknown> =>
      new Promise(
        (
          resolve: (result: Promise<any> | any) => void,
          reject: (error?: any) => void,
        ) => {
          nullthrows(this.workers.get(childId)).call({
            ...request,
            resolve,
            reject,
            retries: 0,
          });
        },
      ),
    runHandle: (handle: Handle, args: Array<any>): Promise<unknown> =>
      this.workerApi.callChild(nullthrows(handle.childId), {
        handle: handle.id,
        args,
      }),
    getSharedReference: (ref: SharedReference) =>
      this.sharedReferences.get(ref),
    resolveSharedReference: (value: unknown) =>
      this.sharedReferencesByValue.get(value),
  };

  warmupWorker(method: string, args: Array<any>): void {
    // Workers are already stopping
    if (this.ending) {
      return;
    }

    // Workers are not warmed up yet.
    // Send the job to a remote worker in the background,
    // but use the result from the local worker - it will be faster.
    let promise = this.addCall(method, [...args, true]);
    if (promise) {
      promise
        .then(() => {
          this.warmWorkers++;
          if (this.warmWorkers >= this.workers.size) {
            this.emit('warmedup');
          }
        })
        .catch(() => {});
    }
  }

  shouldStartRemoteWorkers(): boolean {
    return (
      this.options.maxConcurrentWorkers > 0 || !this.options.useLocalWorker
    );
  }

  createHandle(method: string, useMainThread: boolean = false): HandleFunction {
    if (!this.options.useLocalWorker) {
      useMainThread = false;
    }

    return async (...args) => {
      // Child process workers are slow to start (~600ms).
      // While we're waiting, just run on the main thread.
      // This significantly speeds up startup time.
      if (this.shouldUseRemoteWorkers() && !useMainThread) {
        return this.addCall(method, [...args, false]);
      } else {
        if (this.options.warmWorkers && this.shouldStartRemoteWorkers()) {
          this.warmupWorker(method, args);
        }

        let processedArgs;
        if (!useMainThread) {
          processedArgs = restoreDeserializedObject(
            prepareForSerialization([...args, false]),
          );
        } else {
          processedArgs = args;
        }

        if (this.localWorkerInit != null) {
          await this.localWorkerInit;
          this.localWorkerInit = null;
        }
        return this.localWorker[method](this.workerApi, ...processedArgs);
      }
    };
  }

  onError(
    error: ErrorWithCode,
    worker: Worker,
  ): undefined | Promise<undefined> {
    // Handle ipc errors
    if (error.code === 'ERR_IPC_CHANNEL_CLOSED') {
      return this.stopWorker(worker);
    } else {
      logger.error(error, '@atlaspack/workers');
    }
  }

  startChild() {
    let worker = new Worker({
      forcedKillTime: this.options.forcedKillTime,
      backend: this.options.backend,
      shouldPatchConsole: this.options.shouldPatchConsole,
      shouldTrace: this.options.shouldTrace,
      sharedReferences: this.sharedReferences,
    });

    worker.fork(nullthrows(this.options.workerPath));

    worker.on('request', (data) => this.processRequest(data, worker));

    worker.on('ready', () => {
      this.readyWorkers++;
      if (this.readyWorkers === this.options.maxConcurrentWorkers) {
        this.emit('ready');
      }
      this.processQueue();
    });
    worker.on('response', () => this.processQueue());

    worker.on('error', (err) => this.onError(err, worker));
    worker.once('exit', () => this.stopWorker(worker));

    this.workers.set(worker.id, worker);
  }

  async stopWorker(worker: Worker): Promise<void> {
    if (!worker.stopped) {
      this.workers.delete(worker.id);

      worker.isStopping = true;

      if (worker.calls.size) {
        for (let call of worker.calls.values()) {
          call.retries++;
          this.callQueue.unshift(call);
        }
      }

      worker.calls.clear();

      await worker.stop();

      // Process any requests that failed and start a new worker
      this.processQueue();
    }
  }

  processQueue(): void {
    if (this.ending || !this.callQueue.length) return;

    if (this.workers.size < this.options.maxConcurrentWorkers) {
      this.startChild();
    }

    let workers = [...this.workers.values()].sort(
      (a, b) => a.calls.size - b.calls.size,
    );

    for (let worker of workers) {
      if (!this.callQueue.length) {
        break;
      }

      if (!worker.ready || worker.stopped || worker.isStopping) {
        continue;
      }

      if (worker.calls.size < this.options.maxConcurrentCallsPerWorker) {
        this.callWorker(worker, this.callQueue.shift());
      }
    }
  }

  async callWorker(worker: Worker, call: WorkerCall): Promise<void> {
    for (let ref of this.sharedReferences.keys()) {
      if (!worker.sentSharedReferences.has(ref)) {
        await worker.sendSharedReference(
          ref,
          this.getSerializedSharedReference(ref),
        );
      }
    }

    worker.call(call);
  }

  async processRequest(
    data: {
      location: FilePath;
    } & Partial<WorkerRequest>,
    worker?: Worker,
  ): Promise<string | null | undefined> {
    let {method, args, location, awaitResponse, idx, handle: handleId} = data;
    let mod;
    if (handleId != null) {
      mod = nullthrows(this.handles.get(handleId)?.fn);
    } else if (location) {
      if (process.browser) {
        if (location === '@atlaspack/workers/src/bus.js') {
          mod = bus as any;
        } else {
          throw new Error('No dynamic require possible: ' + location);
        }
      } else {
        mod = require(location);
      }
    } else {
      throw new Error('Unknown request');
    }

    const responseFromContent = (content: any): WorkerDataResponse => ({
      idx,
      type: 'response',
      contentType: 'data',
      content,
    });

    const errorResponseFromError = (e: Error): WorkerErrorResponse => ({
      idx,
      type: 'response',
      contentType: 'error',
      content: anyToDiagnostic(e),
    });

    let result;
    if (method == null) {
      try {
        result = responseFromContent(await mod(...args));
      } catch (e: any) {
        result = errorResponseFromError(e);
      }
    } else {
      // ESModule default interop
      if (mod.__esModule && !mod[method] && mod.default) {
        mod = mod.default;
      }

      try {
        result = responseFromContent(await mod[method](...args));
      } catch (e: any) {
        result = errorResponseFromError(e);
      }
    }

    if (awaitResponse) {
      if (worker) {
        worker.send(result);
      } else {
        if (result.contentType === 'error') {
          throw new ThrowableDiagnostic({diagnostic: result.content});
        }
        return result.content;
      }
    }
  }

  addCall(method: string, args: Array<any>): Promise<any> {
    if (this.ending) {
      throw new Error('Cannot add a worker call if workerfarm is ending.');
    }

    return new Promise(
      (
        resolve: (result: Promise<any> | any) => void,
        reject: (error?: any) => void,
      ) => {
        this.callQueue.push({
          method,
          args: args,
          retries: 0,
          resolve,
          reject,
        });
        this.processQueue();
      },
    );
  }

  async end(): Promise<void> {
    this.ending = true;

    await Promise.all(
      Array.from(this.workers.values()).map((worker) =>
        this.stopWorker(worker),
      ),
    );

    for (let handle of this.handles.values()) {
      handle.dispose();
    }
    this.handles = new Map();
    this.sharedReferences = new Map();
    this.sharedReferencesByValue = new Map();

    this.ending = false;
  }

  startMaxWorkers(): void {
    // Starts workers until the maximum is reached
    if (this.workers.size < this.options.maxConcurrentWorkers) {
      let toStart = this.options.maxConcurrentWorkers - this.workers.size;
      while (toStart--) {
        this.startChild();
      }
    }
  }

  shouldUseRemoteWorkers(): boolean {
    return (
      !this.options.useLocalWorker ||
      ((this.warmWorkers >= this.workers.size || !this.options.warmWorkers) &&
        this.options.maxConcurrentWorkers > 0)
    );
  }

  createReverseHandle(fn: HandleFunction): Handle {
    let handle = new Handle({fn});
    this.handles.set(handle.id, handle);
    return handle;
  }

  createSharedReference(
    value: unknown,
    isCacheable: boolean = true,
  ): {
    ref: SharedReference;
    dispose(): Promise<unknown>;
  } {
    let ref = referenceId++;
    this.sharedReferences.set(ref, value);
    this.sharedReferencesByValue.set(value, ref);
    if (!isCacheable) {
      this.serializedSharedReferences.set(ref, null);
    }

    return {
      ref,
      dispose: () => {
        this.sharedReferences.delete(ref);
        this.sharedReferencesByValue.delete(value);
        this.serializedSharedReferences.delete(ref);

        let promises: Array<Promise<any>> = [];
        for (let worker of this.workers.values()) {
          if (!worker.sentSharedReferences.has(ref)) {
            continue;
          }

          worker.sentSharedReferences.delete(ref);
          promises.push(
            new Promise(
              (
                resolve: (result: Promise<any> | any) => void,
                reject: (error?: any) => void,
              ) => {
                worker.call({
                  method: 'deleteSharedReference',
                  args: [ref],
                  resolve,
                  reject,
                  skipReadyCheck: true,
                  retries: 0,
                });
              },
            ),
          );
        }
        return Promise.all(promises);
      },
    };
  }

  getSerializedSharedReference(ref: SharedReference): ArrayBuffer {
    let cached = this.serializedSharedReferences.get(ref);
    if (cached) {
      return cached;
    }

    let value = this.sharedReferences.get(ref);
    let buf = serialize(value).buffer;

    // If the reference was created with the isCacheable option set to false,
    // serializedSharedReferences will contain `null` as the value.
    if (cached !== null) {
      this.serializedSharedReferences.set(ref, buf);
    }

    return buf;
  }

  async startProfile() {
    let promises: Array<Promise<unknown> | Promise<any>> = [];
    for (let worker of this.workers.values()) {
      promises.push(
        new Promise(
          (
            resolve: (result: Promise<any> | any) => void,
            reject: (error?: any) => void,
          ) => {
            worker.call({
              method: 'startProfile',
              args: [],
              resolve,
              reject,
              retries: 0,
              skipReadyCheck: true,
            });
          },
        ),
      );
    }

    this.profiler = new SamplingProfiler();

    promises.push(this.profiler.startProfiling());
    await Promise.all(promises);
  }

  async endProfile() {
    if (!this.profiler) {
      return;
    }

    let promises = [this.profiler.stopProfiling()];
    let names = ['Master'];

    for (let worker of this.workers.values()) {
      names.push('Worker ' + worker.id);
      promises.push(
        new Promise(
          (
            resolve: (result: Promise<any> | any) => void,
            reject: (error?: any) => void,
          ) => {
            worker.call({
              method: 'endProfile',
              args: [],
              resolve,
              reject,
              retries: 0,
              skipReadyCheck: true,
            });
          },
        ),
      );
    }

    var profiles = await Promise.all(promises);
    let trace = new Trace();
    let filename = `profile-${getTimeId()}.trace`;
    let stream = trace.pipe(fs.createWriteStream(filename));

    for (let profile of profiles) {
      trace.addCPUProfile(names.shift(), profile);
    }

    trace.flush();
    await new Promise((resolve: (result: Promise<never>) => void) => {
      stream.once('finish', resolve);
    });

    logger.info({
      origin: '@atlaspack/workers',
      message: md`Wrote profile to ${filename}`,
    });
  }

  async callAllWorkers(method: string, args: Array<any>) {
    let promises: Array<Promise<unknown> | Promise<any>> = [];
    for (let worker of this.workers.values()) {
      promises.push(
        new Promise(
          (
            resolve: (result: Promise<any> | any) => void,
            reject: (error?: any) => void,
          ) => {
            worker.call({
              method,
              args,
              resolve,
              reject,
              retries: 0,
            });
          },
        ),
      );
    }

    promises.push(this.localWorker[method](this.workerApi, ...args));
    await Promise.all(promises);
  }

  async takeHeapSnapshot() {
    let snapshotId = getTimeId();

    try {
      let snapshotPaths = await Promise.all(
        [...this.workers.values()].map(
          (worker) =>
            new Promise(
              (
                resolve: (result: Promise<any> | any) => void,
                reject: (error?: any) => void,
              ) => {
                worker.call({
                  method: 'takeHeapSnapshot',
                  args: [snapshotId],
                  resolve,
                  reject,
                  retries: 0,
                  skipReadyCheck: true,
                });
              },
            ),
        ),
      );

      logger.info({
        origin: '@atlaspack/workers',
        message: md`Wrote heap snapshots to the following paths:\n${snapshotPaths.join(
          '\n',
        )}`,
      });
    } catch {
      logger.error({
        origin: '@atlaspack/workers',
        message: 'Unable to take heap snapshots. Note: requires Node 11.13.0+',
      });
    }
  }

  static getNumWorkers(): number {
    return process.env.ATLASPACK_WORKERS
      ? parseInt(process.env.ATLASPACK_WORKERS, 10)
      : Math.min(4, Math.ceil(cpuCount() / 2));
  }

  static isWorker(): boolean {
    return !!child;
  }

  static getWorkerApi(): {
    callMaster: (
      request: CallRequest,
      awaitResponse?: boolean | null | undefined,
    ) => Promise<unknown>;
    createReverseHandle: (fn: (...args: Array<any>) => unknown) => Handle;
    getSharedReference: (ref: SharedReference) => unknown;
    resolveSharedReference: (value: unknown) => undefined | SharedReference;
    runHandle: (handle: Handle, args: Array<any>) => Promise<unknown>;
  } {
    invariant(
      child != null,
      'WorkerFarm.getWorkerApi can only be called within workers',
    );
    return child.workerApi;
  }

  static getConcurrentCallsPerWorker(
    defaultValue: number = DEFAULT_MAX_CONCURRENT_CALLS,
  ): number {
    return (
      parseInt(process.env.ATLASPACK_MAX_CONCURRENT_CALLS, 10) || defaultValue
    );
  }
}

function getTimeId() {
  let now = new Date();
  return (
    String(now.getFullYear()) +
    String(now.getMonth() + 1).padStart(2, '0') +
    String(now.getDate()).padStart(2, '0') +
    '-' +
    String(now.getHours()).padStart(2, '0') +
    String(now.getMinutes()).padStart(2, '0') +
    String(now.getSeconds()).padStart(2, '0')
  );
}
