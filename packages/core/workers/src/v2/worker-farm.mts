import {EventEmitter} from 'node:events';
import {DEFAULT_WORKER_TIMEOUT} from './constants.mts';
import {WorkerThread} from './worker-thread.mts';
import type {WorkerThreadOptions} from './worker-thread.mts';
import type {IWorker, WorkerStatus, Transferrable} from './worker-interface.mts';

export type WorkerFarmWorkerStatus = {
  totalTasks: number;
  workers: Array<{
    tasks: number;
    status: WorkerStatus;
  }>;
};

export type WorkerFarmOptions = {
  workerCount?: number;
  workerPath?: string;
  workerTimeout?: number;
};

export class WorkerFarm extends EventEmitter {
  #workers: Array<IWorker>;

  constructor(options: WorkerFarmOptions) {
    super();
    const resolvedOptions = WorkerFarm.resolveOptions(options);
    this.#workers = WorkerFarm.spawnWorkers(resolvedOptions);
    this.onReady().then(() => this.emit('ready'));
  }

  static resolveOptions(
    options: WorkerFarmOptions,
  ): Required<WorkerFarmOptions> {
    if (!options.workerPath) {
      throw new Error('No worker path supplied');
    }
    const resolved: Required<WorkerFarmOptions> = {
      workerCount: options.workerCount || 1,
      workerPath: options.workerPath,
      workerTimeout: options.workerTimeout || DEFAULT_WORKER_TIMEOUT,
    };
    return resolved;
  }

  static spawnWorkers(
    options: WorkerThreadOptions & {workerCount: number},
  ): Array<WorkerThread> {
    const workers: Array<WorkerThread> = [];
    for (let i = 0; i < options.workerCount; i++) {
      workers.push(
        new WorkerThread({
          workerTimeout: options.workerTimeout,
          workerPath: options.workerPath,
        }),
      );
    }
    return workers;
  }

  async onReady(): Promise<void> {
    await Promise.all(this.#workers.map((w) => w.onReady()));
  }

  status(): WorkerFarmWorkerStatus {
    return {
      totalTasks: this.tasks(),
      workers: this.#workers.map((w) => ({
        tasks: w.tasks(),
        status: w.status(),
      })),
    };
  }

  tasks(): number {
    let i = 0;
    for (const worker of this.#workers) {
      i += worker.tasks();
    }
    return i;
  }

  run<R = unknown, A extends Array<Transferrable> = any[]>(...args: A): Promise<R> {
    return this.exec('run', args);
  }

  exec<R = unknown, A extends Array<Transferrable> = any[]>(
    methodName: string,
    args: A,
  ): Promise<R> {
    return this.#workers[this.#next()].exec(methodName, args) as Promise<R>;
  }

  async flush(): Promise<void> {
    await Promise.all(this.#workers.map((w) => w.flush()));
  }

  /** @description gracefully terminate all workers and wait for
   * any async actions to complete before resolving */
  async end(): Promise<void> {
    await Promise.all(this.#workers.map((w) => w.end()));
  }

  /** @description Find the next worker with 0 active tasks
   * or use the one with the lowest number of active tasks */
  #next() {
    const tasks: Array<number> = Array(this.#workers.length);
    for (const [i, worker] of this.#workers.entries()) {
      const n = worker.tasks();
      if (n === 0) {
        return i;
      }
      tasks[i] = worker.tasks();
    }
    return tasks.indexOf(Math.min(...tasks));
  }
}
