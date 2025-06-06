import {EventEmitter} from 'node:events';
import {DEFAULT_WORKER_TIMEOUT} from './constants.mts';
import {WorkerThread} from './threads/worker-thread.mts';
import type {WorkerThreadOptions} from './threads/worker-thread.mts';
import type {
  IWorker,
  WorkerStatus,
  TransferItem,
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
  #referenceId: number;
  #sharedReferences: Map<number, any>;
  #sharedReferencesByValue: Map<any, number>;

  constructor(options: WorkerFarmOptions) {
    super();
    const resolvedOptions = WorkerFarm.resolveOptions(options);
    this.#reverseHandles = [];
    this.#referenceId = 0;
    this.#sharedReferences = new Map();
    this.#sharedReferencesByValue = new Map();
    this.#workers = WorkerFarm.spawnWorkers({
      reverseHandles: this.#reverseHandles,
      ...resolvedOptions,
    });
    this.onReady().then(() => this.emit('ready'));
  }

  /** @description creates the default options */
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

  /** @description spawns a collection of workers */
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

  /** @description will resolve when all of the workers have started */
  async onReady(): Promise<void> {
    await Promise.all(this.#workers.map((w) => w.onReady()));
  }

  /** @description provides information on the current status of the worker farm */
  status(): WorkerFarmWorkerStatus {
    let totalTasks = 0;
    for (const worker of this.#workers) {
      totalTasks += worker.tasks();
    }
    return {
      totalTasks,
      handles: this.#reverseHandles.length,
      workers: this.#workers.map((w) => ({
        tasks: w.tasks(),
        status: w.status(),
      })),
    };
  }

  /**
   * @description starts a task on the worker thread. This will run an
   * exported function called "run" within the worker
   */
  run<R = unknown, A extends Array<TransferItem> = any[]>(
    ...args: A
  ): Promise<R> {
    return this.exec('run', args);
  }

  /** @description start a task in the worker thread and return the value */
  exec<R = unknown, A extends Array<TransferItem> = any[]>(
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

  /** @description wait for the current tasks in flight to complete */
  async flush(): Promise<void> {
    await Promise.all(this.#workers.map((w) => w.flush()));
  }

  /**
   * @description gracefully terminate all workers and wait for
   * any async actions to complete before resolving */
  async end(): Promise<void> {
    await Promise.all(this.#workers.map((w) => w.end()));
    this.#reverseHandles.length = 0; // clear the array
  }

  /**
   * @description creates a reference to a value that is transferred
   * lazily and synchronized with workers
   */
  createSharedReference<T>(
    value: T,
    isCacheable: boolean = true,
  ): SharableReference<T> {
    let id = this.#referenceId++;
    this.#sharedReferences.set(id, value);
    this.#sharedReferencesByValue.set(value, id);
    return new SharableReference(
      id,
      value,
      this.#sharedReferences,
      this.#sharedReferencesByValue,
    );
  }

  /**
   * @description A function defined on the orchestrator thread which can be
   * executed within the worker thread via the worker's API
   */
  createReverseHandle<R = unknown, A extends Array<TransferItem> = any[]>(
    handleFunc: HandleFunc<R, A>,
  ): HandleRef {
    return new HandleRef(this.#reverseHandles.push(handleFunc) - 1);
  }

  /**
   * @description Find the next worker with 0 active tasks
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

export class SharableReference<T> {
  ref: number;
  #value: T;
  #sharedReferences: Map<number, any>;
  #sharedReferencesByValue: Map<any, number>;

  constructor(
    ref: number,
    value: T,
    sharedReferences: Map<number, any>,
    sharedReferencesByValue: Map<any, number>,
  ) {
    this.ref = ref;
    this.#value = value;
    this.#sharedReferences = sharedReferences;
    this.#sharedReferencesByValue = sharedReferencesByValue;
  }

  dispose() {
    this.#sharedReferences.delete(this.ref);
    this.#sharedReferencesByValue.delete(this.#value);
  }
}
