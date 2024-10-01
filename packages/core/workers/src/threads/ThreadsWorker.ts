import type {
  WorkerImpl,
  MessageHandler,
  ErrorHandler,
  ExitHandler,
  WorkerMessage,
} from '../types';
import {Worker} from 'worker_threads';
import path from 'path';
import {
  prepareForSerialization,
  restoreDeserializedObject,
} from '@atlaspack/core';

const WORKER_PATH = path.join(__dirname, 'ThreadsChild.ts');

// @ts-expect-error - TS2420 - Class 'ThreadsWorker' incorrectly implements interface 'WorkerImpl'.
export default class ThreadsWorker implements WorkerImpl {
  execArgv: any;
  onMessage: MessageHandler;
  onError: ErrorHandler;
  onExit: ExitHandler;
  // @ts-expect-error - TS2564 - Property 'worker' has no initializer and is not definitely assigned in the constructor.
  worker: Worker;

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
    this.worker = new Worker(WORKER_PATH, {
      execArgv: this.execArgv,
      env: process.env,
    });

    this.worker.on('message', (data) => this.handleMessage(data));
    this.worker.on('error', this.onError);
    this.worker.on('exit', this.onExit);

    return new Promise<undefined>(
      (resolve: (result: Promise<undefined> | undefined) => void) => {
        this.worker.on('online', resolve);
      },
    );
  }

  stop(): Promise<void> {
    // In node 12, this returns a promise, but previously it accepted a callback
    // TODO: Pass a callback in earlier versions of Node
    // @ts-expect-error - TS2322 - Type 'Promise<number>' is not assignable to type 'Promise<void>'.
    return Promise.resolve(this.worker.terminate());
  }

  handleMessage(data: WorkerMessage) {
    this.onMessage(restoreDeserializedObject(data));
  }

  send(data: WorkerMessage) {
    this.worker.postMessage(prepareForSerialization(data));
  }
}
