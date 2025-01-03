// @flow strict-local

/* eslint-disable no-console */

/*!
 * Atlaspack V3 delegates work to node.js worker threads.
 *
 * Starting-up each worker is relatively expensive, in particular when atlaspack
 * is running in development mode, in which case each worker will transpile the
 * project on startup.
 *
 * This "WorkerPool" mitigates this problem by reusing worker threads across
 * builds.
 */
import path from 'path';
// $FlowFixMe Missing types
import {setTimeout} from 'timers/promises';
import {Worker} from 'worker_threads';

const WORKER_PATH = path.join(__dirname, 'worker', 'index.js');

export function waitForMessage<T>(
  worker: Worker,
  type: string,
  signal: AbortSignal,
): Promise<T> {
  return new Promise((resolve) => {
    const onMessage = (message: T & {|type: string|}) => {
      if (message.type === type) {
        resolve(message);
        worker.off('message', onMessage);
      }
    };

    worker.on('message', onMessage);

    const onAbort = () => {
      signal.removeEventListener('abort', onAbort);
      worker.off('message', onMessage);
    };

    signal.addEventListener('abort', onAbort);
  });
}

export class WorkerPool {
  #workerPool: Worker[] = [];
  #usedWorkers: Set<number> = new Set();
  #workerPath: string;

  constructor(workerPath: string = WORKER_PATH) {
    this.#workerPath = workerPath;

    // $FlowFixMe
    this[Symbol.dispose] = () => {
      this.shutdown();
    };
  }

  /**
   * Find a worker thread that is free to use or create a new one.
   *
   * Then register the `tx_worker` channel ID with the worker thread.
   */
  registerWorker(tx_worker: number): number {
    const availableIndex = this.#workerPool.findIndex(
      (worker, index) => !this.#usedWorkers.has(index),
    );

    const [workerId, worker] =
      availableIndex !== -1
        ? [availableIndex, this.#workerPool[availableIndex]]
        : this.#createWorker();

    this.#bootWorker(worker, tx_worker).catch((err) => {
      // eslint-disable-next-line no-console
      console.error('Worker failed, retrying to create it...', err);
      this.#workerPool[workerId] = new Worker(this.#workerPath, {
        workerData: {attempt: 2},
      });

      this.#bootWorker(this.#workerPool[workerId], tx_worker).catch((err) => {
        console.error('Worker failed to start, the build may hang:', err);
      });
    });

    this.#usedWorkers.add(workerId);

    return workerId;
  }

  /**
   * Release a set of workers back into the pool for re-use
   */
  releaseWorkers(ids: number[]) {
    for (let id of ids) {
      this.#usedWorkers.delete(id);
    }
  }

  /**
   * Terminate all worker threads and reset state.
   */
  shutdown() {
    for (let worker of this.#workerPool) {
      worker.terminate();
    }
    this.#usedWorkers.clear();
    this.#workerPool = [];
  }

  getStats(): {|totalWorkers: number, workersInUse: number|} {
    return {
      totalWorkers: this.#workerPool.length,
      workersInUse: this.#usedWorkers.size,
    };
  }

  /**
   * Get the worker thread. Used for testing.
   */
  getWorker(workerId: number): Worker {
    if (!this.#workerPool[workerId]) {
      throw new Error('Worker does not exist');
    }
    return this.#workerPool[workerId];
  }

  async #bootWorker(worker: Worker, tx_worker: number): Promise<void> {
    const controller = new AbortController();
    const signal = controller.signal;

    const workerError = new Promise((_, reject) => {
      const onError = (err: Error) => {
        reject(err);
      };

      worker.once('error', onError);

      const onAbort = () => {
        signal.removeEventListener('abort', onAbort);
        worker.off('error', onError);
      };

      signal.addEventListener('abort', onAbort);
    });

    const workerReady = waitForMessage(worker, 'workerRegistered', signal);

    worker.postMessage({type: 'registerWorker', tx_worker});

    try {
      await Promise.race([
        setTimeout(5000, {signal}),
        workerError,
        workerReady,
      ]);
    } finally {
      controller.abort();
    }
  }

  #createWorker(): [number, Worker] {
    const worker = new Worker(this.#workerPath, {workerData: {attempt: 1}});
    const workerId = this.#workerPool.length;
    this.#workerPool.push(worker);
    return [workerId, worker];
  }
}

export const workerPool: WorkerPool = new WorkerPool();
