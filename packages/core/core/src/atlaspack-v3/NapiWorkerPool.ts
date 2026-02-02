import type {NapiWorkerPool as INapiWorkerPool} from '@atlaspack/types';
import {Worker} from 'worker_threads';
import path from 'path';
import process from 'process';
// @ts-expect-error TS2724
import type {Transferable} from '@atlaspack/rust';
import {getAvailableThreads, clearTransferableRegistry} from '@atlaspack/rust';

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

            // Set up a message handler that only resolves on 'stateCleared'
            // and ignores all other messages (like the initial napiWorker Transferable)
            const messageHandler = (message: unknown) => {
              if (message === 'stateCleared') {
                worker.removeListener('message', messageHandler);
                res();
              } else {
                // Log unexpected messages for debugging
                // eslint-disable-next-line no-console
                console.warn(
                  `[NapiWorkerPool] Received unexpected message during clearAllWorkerState: ${JSON.stringify(message)} (type: ${typeof message})`,
                );
                // Keep listening for 'stateCleared' - don't remove the listener
              }
            };

            worker.on('message', messageHandler);
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

  /**
   * Shuts down the worker pool by terminating all worker threads.
   *
   * This method also clears the JsTransferable registry to release
   * Arc<NodejsWorker> references that would otherwise persist in memory.
   */
  async shutdown(): Promise<void> {
    const terminatePromises: Promise<number>[] = [];

    for (const worker of this.#workers) {
      terminatePromises.push(worker.terminate());
    }

    // Wait for all workers to terminate
    await Promise.all(terminatePromises);

    // Clear the workers array so we don't try to use them again
    this.#workers = [];
    this.#napiWorkers = [];

    // Clear the JsTransferable registry to release Arc<NodejsWorker> references.
    // Without this, the Rust-side NodejsWorker instances would persist in memory
    // even though the JS worker threads are terminated.
    if (clearTransferableRegistry) {
      clearTransferableRegistry();
    }
  }
}
