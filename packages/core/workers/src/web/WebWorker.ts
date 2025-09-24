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
} from '@atlaspack/build-cache';
import {makeDeferredWithPromise} from '@atlaspack/utils';

let id = 0;

// @ts-expect-error This is actually a module
export let WORKER_PATH = new URL('./WebChild.js', import.meta.url);
if (process.env.ATLASPACK_REGISTER_USE_SRC === 'true') {
  // @ts-expect-error This is actually a module
  WORKER_PATH = new URL('./WebChild.ts', import.meta.url);
}

// @ts-expect-error TS2420
export default class WebWorker implements WorkerImpl {
  execArgv: any;
  onMessage: MessageHandler;
  onError: ErrorHandler;
  onExit: ExitHandler;
  // @ts-expect-error TS2564
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
    // @ts-expect-error TS1470
    this.worker = new Worker(new URL('./WebChild.js', import.meta.url), {
      name: `Parcel Worker ${id++}`,
      type: 'module',
    });

    let {deferred, promise} = makeDeferredWithPromise();

    this.worker.onmessage = ({data}) => {
      if (data === 'online') {
        // @ts-expect-error TS2554
        deferred.resolve();
        return;
      }

      this.handleMessage(data);
    };
    // @ts-expect-error TS2322
    this.worker.onerror = this.onError;
    // Web workers can't crash or intentionally stop on their own, apart from stop() below
    // this.worker.on('exit', this.onExit);

    // @ts-expect-error TS2322
    return promise;
  }

  stop(): Promise<void> {
    if (!this.stopping) {
      // @ts-expect-error TS2322
      this.stopping = (async () => {
        this.worker.postMessage('stop');
        let {deferred, promise} = makeDeferredWithPromise();
        this.worker.addEventListener('message', ({data}: MessageEvent) => {
          if (data === 'stopped') {
            // @ts-expect-error TS2554
            deferred.resolve();
          }
        });
        await promise;
        this.worker.terminate();
        this.onExit(0);
      })();
    }
    // @ts-expect-error TS2322
    return this.stopping;
  }

  handleMessage(data: WorkerMessage) {
    this.onMessage(restoreDeserializedObject(data));
  }

  send(data: WorkerMessage) {
    this.worker.postMessage(prepareForSerialization(data));
  }
}
