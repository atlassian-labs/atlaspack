// @ts-expect-error - TS2307 - Cannot find module 'flow-to-typescript-codemod' or its corresponding type declarations.
import {Flow} from 'flow-to-typescript-codemod';

import type {
  CallRequest,
  WorkerDataResponse,
  WorkerErrorResponse,
  WorkerMessage,
  WorkerRequest,
  WorkerResponse,
  ChildImpl,
} from './types';
import type {Async, IDisposable} from '@atlaspack/types-internal';
import type {SharedReference} from './WorkerFarm';

// @ts-expect-error - TS7016 - Could not find a declaration file for module './core-worker'. '/home/ubuntu/parcel/packages/core/workers/src/core-worker.js' implicitly has an 'any' type.
import * as coreWorker from './core-worker';
import invariant from 'assert';
import nullthrows from 'nullthrows';
import Logger, {patchConsole, unpatchConsole} from '@atlaspack/logger';
import ThrowableDiagnostic, {anyToDiagnostic} from '@atlaspack/diagnostic';
import {deserialize} from '@atlaspack/core';
import bus from './bus';
import {SamplingProfiler, tracer} from '@atlaspack/profiler';
import _Handle from './Handle';

// The import of './Handle' should really be imported eagerly (with @babel/plugin-transform-modules-commonjs's lazy mode).
const Handle = _Handle;

type ChildCall = WorkerRequest & {
  resolve: (result: Promise<any> | any) => void;
  reject: (error?: any) => void;
};

export class Child {
  callQueue: Array<ChildCall> = [];
  childId: number | null | undefined;
  maxConcurrentCalls: number = 10;
  module: any | null | undefined;
  responseId: number = 0;
  responseQueue: Map<number, ChildCall> = new Map();
  loggerDisposable: IDisposable;
  tracerDisposable: IDisposable;
  child: ChildImpl;
  profiler: SamplingProfiler | null | undefined;
  // @ts-expect-error - TS2749 - 'Handle' refers to a value, but is being used as a type here. Did you mean 'typeof Handle'?
  handles: Map<number, Handle> = new Map();
  sharedReferences: Map<SharedReference, unknown> = new Map();
  sharedReferencesByValue: Map<unknown, SharedReference> = new Map();

  constructor(ChildBackend: Flow.Class<ChildImpl>) {
    this.child = new ChildBackend(
      (m: WorkerMessage) => {
        this.messageListener(m);
      },
      () => this.handleEnd(),
    );

    // Monitior all logging events inside this child process and forward to
    // the main process via the bus.
    this.loggerDisposable = Logger.onLog((event) => {
      bus.emit('logEvent', event);
    });
    // .. and do the same for trace events
    this.tracerDisposable = tracer.onTrace((event) => {
      bus.emit('traceEvent', event);
    });
  }

  workerApi: {
    callMaster: (
      request: CallRequest,
      awaitResponse?: boolean | null | undefined,
    ) => Promise<unknown>;
    // @ts-expect-error - TS2749 - 'Handle' refers to a value, but is being used as a type here. Did you mean 'typeof Handle'?
    createReverseHandle: (fn: (...args: Array<any>) => unknown) => Handle;
    getSharedReference: (ref: SharedReference) => unknown;
    resolveSharedReference: (value: unknown) => undefined | SharedReference;
    // @ts-expect-error - TS2749 - 'Handle' refers to a value, but is being used as a type here. Did you mean 'typeof Handle'?
    runHandle: (handle: Handle, args: Array<any>) => Promise<unknown>;
  } = {
    callMaster: (
      request: CallRequest,
      awaitResponse: boolean | null = true,
    ): Promise<unknown> => this.addCall(request, awaitResponse),
    // @ts-expect-error - TS2749 - 'Handle' refers to a value, but is being used as a type here. Did you mean 'typeof Handle'?
    createReverseHandle: (fn: (...args: Array<any>) => unknown): Handle =>
      this.createReverseHandle(fn),
    // @ts-expect-error - TS2749 - 'Handle' refers to a value, but is being used as a type here. Did you mean 'typeof Handle'?
    runHandle: (handle: Handle, args: Array<any>): Promise<unknown> =>
      this.workerApi.callMaster({handle: handle.id, args}, true),
    getSharedReference: (ref: SharedReference) =>
      this.sharedReferences.get(ref),
    resolveSharedReference: (value: unknown) =>
      this.sharedReferencesByValue.get(value),
  };

  messageListener(message: WorkerMessage): Async<void> {
    if (message.type === 'response') {
      return this.handleResponse(message);
    } else if (message.type === 'request') {
      return this.handleRequest(message);
    }
  }

  send(data: WorkerMessage): void {
    this.child.send(data);
  }

  async childInit(module: string, childId: number): Promise<void> {
    // @ts-expect-error - TS2339 - Property 'browser' does not exist on type 'Process'.
    if (process.browser) {
      if (module === '@atlaspack/core/src/worker.ts') {
        this.module = coreWorker;
      } else {
        throw new Error('No dynamic require possible: ' + module);
      }
    } else {
      this.module = require(module);
    }
    this.childId = childId;

    if (this.module.childInit != null) {
      await this.module.childInit();
    }
  }

  async handleRequest(data: WorkerRequest): Promise<void> {
    let {idx, method, args, handle: handleId} = data;
    let child = nullthrows(data.child);

    const responseFromContent = (content: any): WorkerDataResponse => ({
      idx,
      child,
      type: 'response',
      contentType: 'data',
      content,
    });

    const errorResponseFromError = (e: Error): WorkerErrorResponse => ({
      idx,
      child,
      type: 'response',
      contentType: 'error',
      content: anyToDiagnostic(e),
    });

    let result;
    if (handleId != null) {
      try {
        let fn = nullthrows(this.handles.get(handleId)?.fn);
        result = responseFromContent(fn(...args));
      } catch (e: any) {
        result = errorResponseFromError(e);
      }
    } else if (method === 'childInit') {
      try {
        let [moduleName, childOptions] = args;
        if (childOptions.shouldPatchConsole) {
          patchConsole();
        } else {
          unpatchConsole();
        }

        if (childOptions.shouldTrace) {
          tracer.enable();
        }

        result = responseFromContent(await this.childInit(moduleName, child));
      } catch (e: any) {
        result = errorResponseFromError(e);
      }
    } else if (method === 'startProfile') {
      this.profiler = new SamplingProfiler();
      try {
        result = responseFromContent(await this.profiler.startProfiling());
      } catch (e: any) {
        result = errorResponseFromError(e);
      }
    } else if (method === 'endProfile') {
      try {
        let res = this.profiler ? await this.profiler.stopProfiling() : null;
        result = responseFromContent(res);
      } catch (e: any) {
        result = errorResponseFromError(e);
      }
    } else if (method === 'takeHeapSnapshot') {
      try {
        let v8 = require('v8');
        result = responseFromContent(
          v8.writeHeapSnapshot(
            'heap-' +
              args[0] +
              '-' +
              (this.childId ? 'worker' + this.childId : 'main') +
              '.heapsnapshot',
          ),
        );
      } catch (e: any) {
        result = errorResponseFromError(e);
      }
    } else if (method === 'createSharedReference') {
      let [ref, _value] = args;
      let value =
        _value instanceof ArrayBuffer
          ? // In the case the value is pre-serialized as a buffer,
            // deserialize it.
            deserialize(Buffer.from(_value))
          : _value;
      this.sharedReferences.set(ref, value);
      this.sharedReferencesByValue.set(value, ref);
      result = responseFromContent(null);
    } else if (method === 'deleteSharedReference') {
      let ref = args[0];
      let value = this.sharedReferences.get(ref);
      this.sharedReferencesByValue.delete(value);
      this.sharedReferences.delete(ref);
      result = responseFromContent(null);
    } else {
      try {
        result = responseFromContent(
          // $FlowFixMe
          // @ts-expect-error - TS2538 - Type 'null' cannot be used as an index type. | TS2538 - Type 'undefined' cannot be used as an index type.
          await this.module[method](this.workerApi, ...args),
        );
      } catch (e: any) {
        result = errorResponseFromError(e);
      }
    }

    try {
      this.send(result);
    } catch (e: any) {
      result = this.send(errorResponseFromError(e));
    }
  }

  handleResponse(data: WorkerResponse): void {
    let idx = nullthrows(data.idx);
    let contentType = data.contentType;
    let content = data.content;
    let call = nullthrows(this.responseQueue.get(idx));

    if (contentType === 'error') {
      invariant(typeof content !== 'string');
      call.reject(new ThrowableDiagnostic({diagnostic: content}));
    } else {
      call.resolve(content);
    }

    this.responseQueue.delete(idx);

    // Process the next call
    this.processQueue();
  }

  // Keep in mind to make sure responses to these calls are JSON.Stringify safe
  addCall(
    request: CallRequest,
    awaitResponse: boolean | null = true,
  ): Promise<unknown> {
    let call: ChildCall = {
      ...request,
      type: 'request',
      child: this.childId,
      // $FlowFixMe Added in Flow 0.121.0 upgrade in #4381
      // @ts-expect-error - TS2322 - Type 'boolean | null' is not assignable to type 'boolean | undefined'.
      awaitResponse,
      resolve: () => {},
      reject: () => {},
    };

    let promise;
    if (awaitResponse) {
      promise = new Promise(
        (
          resolve: (result: Promise<any> | any) => void,
          reject: (error?: any) => void,
        ) => {
          call.resolve = resolve;
          call.reject = reject;
        },
      );
    }

    this.callQueue.push(call);
    this.processQueue();

    return promise ?? Promise.resolve();
  }

  sendRequest(call: ChildCall): void {
    let idx;
    if (call.awaitResponse) {
      idx = this.responseId++;
      this.responseQueue.set(idx, call);
    }

    this.send({
      idx,
      child: call.child,
      type: call.type,
      location: call.location,
      handle: call.handle,
      method: call.method,
      args: call.args,
      awaitResponse: call.awaitResponse,
    });
  }

  processQueue(): void {
    if (!this.callQueue.length) {
      return;
    }

    if (this.responseQueue.size < this.maxConcurrentCalls) {
      // @ts-expect-error - TS2345 - Argument of type 'ChildCall | undefined' is not assignable to parameter of type 'ChildCall'.
      this.sendRequest(this.callQueue.shift());
    }
  }

  handleEnd(): void {
    this.loggerDisposable.dispose();
    this.tracerDisposable.dispose();
  }

  // @ts-expect-error - TS2749 - 'Handle' refers to a value, but is being used as a type here. Did you mean 'typeof Handle'?
  createReverseHandle(fn: (...args: Array<any>) => unknown): Handle {
    let handle = new Handle({
      fn,
      childId: this.childId,
    });
    this.handles.set(handle.id, handle);
    return handle;
  }
}
