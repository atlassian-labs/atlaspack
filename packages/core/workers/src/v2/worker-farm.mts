import {EventEmitter} from 'node:events';
import {DEFAULT_WORKER_TIMEOUT} from './constants.mts';
import {WorkerThread} from './worker-thread.mts';
import type {WorkerThreadOptions} from './worker-thread.mts';
import type {
  IWorker,
  WorkerStatus,
  Transferrable,
  HandleFunc,
} from './worker-interface.mts';
import {Serializable} from './worker-interface.mts';
import {HandleRef} from './handle-ref.mts';

export type WorkerFarmWorkerStatus = {
  totalTasks: number;
  handles: number;
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
  #reverseHandles: Array<HandleFunc>;

  constructor(options: WorkerFarmOptions) {
    super();
    const resolvedOptions = WorkerFarm.resolveOptions(options);
    this.#reverseHandles = [];
    this.#workers = WorkerFarm.spawnWorkers({
      reverseHandles: this.#reverseHandles,
      ...resolvedOptions,
    });
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
          reverseHandles: options.reverseHandles,
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
      handles: this.#reverseHandles.length,
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

  run<R = unknown, A extends Array<Transferrable> = any[]>(
    ...args: A
  ): Promise<R> {
    return this.exec('run', args);
  }

  exec<R = unknown, A extends Array<Transferrable> = any[]>(
    methodName: string,
    args: A,
  ): Promise<R> {
    const serdeArgs: Array<number> = [];
    const preparedArgs = args.map((arg, index) =>
      arg instanceof Serializable
        ? serdeArgs.push(index) && arg.serialize()
        : arg,
    );
    return this.#workers[this.#next()].exec(
      methodName,
      preparedArgs,
      serdeArgs,
    ) as Promise<R>;
  }

  async flush(): Promise<void> {
    await Promise.all(this.#workers.map((w) => w.flush()));
  }

  /** @description gracefully terminate all workers and wait for
   * any async actions to complete before resolving */
  async end(): Promise<void> {
    await Promise.all(this.#workers.map((w) => w.end()));
    this.#reverseHandles.length = 0; // clear the array
  }

  createReverseHandle<R = unknown, A extends Array<Transferrable> = any[]>(
    handleFunc: HandleFunc<R, A>,
  ): HandleRef {
    return new HandleRef(this.#reverseHandles.push(handleFunc) - 1);
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
