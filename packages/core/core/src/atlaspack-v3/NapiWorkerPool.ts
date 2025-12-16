import type {NapiWorkerPool as INapiWorkerPool} from '@atlaspack/types';
import {Worker} from 'worker_threads';
import path from 'path';
import process from 'process';
// @ts-expect-error TS2724
import type {Transferable} from '@atlaspack/rust';
import {getAvailableThreads} from '@atlaspack/rust';

const WORKER_PATH = path.join(__dirname, 'worker', 'index.js');
const ATLASPACK_NAPI_WORKERS =
  process.env.ATLASPACK_NAPI_WORKERS &&
  parseInt(process.env.ATLASPACK_NAPI_WORKERS, 10);

export type NapiWorkerPoolOptions = {
  workerCount?: number;
};

export class NapiWorkerPool implements INapiWorkerPool {
  #workers: Worker[];
  #napiWorkers: Array<Promise<Transferable>>;
  #workerCount: number;

  constructor({workerCount}: NapiWorkerPoolOptions = {workerCount: undefined}) {
    // @ts-expect-error TS2322
    this.#workerCount =
      workerCount ??
      ATLASPACK_NAPI_WORKERS ??
      // Default to a maximum of 4 workers as performance worsens beyond that
      // point in most cases
      Math.min(getAvailableThreads(), 4);
    if (!this.#workerCount) {
      // TODO use main thread if workerCount is 0
    }

    this.#workers = [];
    this.#napiWorkers = [];

    for (let i = 0; i < this.#workerCount; i++) {
      let worker = new Worker(WORKER_PATH);
      this.#workers.push(worker);
      this.#napiWorkers.push(
        new Promise((res: (result: Promise<never>) => void) =>
          worker.once('message', res),
        ),
      );
    }
  }

  clearAllWorkerState(): Promise<void[]> {
    return Promise.all(
      this.#workers.map(
        (worker) =>
          new Promise<void>((res) => {
            worker.postMessage('clearState');

            worker.once('message', (message) => {
              if (message == 'stateCleared') {
                res();
              }
            });
          }),
      ),
    );
  }

  workerCount(): number {
    return this.#workerCount;
  }

  getWorkers(): Promise<Array<Transferable>> {
    return Promise.all(this.#napiWorkers);
  }

  shutdown(): void {
    for (const worker of this.#workers) {
      worker.terminate();
    }
  }
}
