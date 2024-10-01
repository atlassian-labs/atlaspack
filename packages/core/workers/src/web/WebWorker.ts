import type {
  WorkerImpl,
  MessageHandler,
  ErrorHandler,
  ExitHandler,
  WorkerMessage,
} from '../types';
import {
  prepareForSerialization,
  restoreDeserializedObject,
} from '@atlaspack/core';
import {makeDeferredWithPromise} from '@atlaspack/utils';

let id = 0;

// @ts-expect-error - TS2420 - Class 'WebWorker' incorrectly implements interface 'WorkerImpl'.
export default class WebWorker implements WorkerImpl {
  execArgv: any;
  onMessage: MessageHandler;
  onError: ErrorHandler;
  onExit: ExitHandler;
  // @ts-expect-error - TS2564 - Property 'worker' has no initializer and is not definitely assigned in the constructor.
  worker: Worker;
  stopping: Promise<undefined> | null | undefined;

  constructor(
    execArgv: any,
    onMessage: MessageHandler,
    onError: ErrorHandler,
    onExit: ExitHandler,
  ) {
    this.execArgv = execArgv;
    this.onMessage = onMessage;
    this.onError = onError;
    this.onExit = onExit;
  }

  start(): Promise<void> {
    // @ts-expect-error - TS1343 - The 'import.meta' meta-property is only allowed when the '--module' option is 'es2020', 'es2022', 'esnext', 'system', 'node12', or 'nodenext'.
    this.worker = new Worker(new URL('./WebChild.js', import.meta.url), {
      name: `Parcel Worker ${id++}`,
      type: 'module',
    });

    let {deferred, promise} = makeDeferredWithPromise();

    this.worker.onmessage = ({data}) => {
      if (data === 'online') {
        // @ts-expect-error - TS2554 - Expected 1 arguments, but got 0.
        deferred.resolve();
        return;
      }

      this.handleMessage(data);
    };
    // @ts-expect-error - TS2322 - Type 'ErrorHandler' is not assignable to type '(this: AbstractWorker, ev: ErrorEvent) => any'.
    this.worker.onerror = this.onError;
    // Web workers can't crash or intentionally stop on their own, apart from stop() below
    // this.worker.on('exit', this.onExit);

    // @ts-expect-error - TS2322 - Type 'Promise<unknown>' is not assignable to type 'Promise<void>'.
    return promise;
  }

  stop(): Promise<void> {
    if (!this.stopping) {
      // @ts-expect-error - TS2322 - Type 'Promise<void>' is not assignable to type 'Promise<undefined>'.
      this.stopping = (async () => {
        this.worker.postMessage('stop');
        let {deferred, promise} = makeDeferredWithPromise();
        this.worker.addEventListener('message', ({data}: MessageEvent) => {
          if (data === 'stopped') {
            // @ts-expect-error - TS2554 - Expected 1 arguments, but got 0.
            deferred.resolve();
          }
        });
        await promise;
        this.worker.terminate();
        this.onExit(0);
      })();
    }
    // @ts-expect-error - TS2322 - Type 'Promise<undefined> | null | undefined' is not assignable to type 'Promise<void>'.
    return this.stopping;
  }

  handleMessage(data: WorkerMessage) {
    this.onMessage(restoreDeserializedObject(data));
  }

  send(data: WorkerMessage) {
    this.worker.postMessage(prepareForSerialization(data));
  }
}
