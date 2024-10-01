import path from 'path';
import {Worker} from 'worker_threads';
import {AtlaspackNapi, AtlaspackNapiOptions} from '@atlaspack/rust';

const WORKER_PATH = path.join(__dirname, 'worker', 'index.js');

export type AtlaspackV3Options = {
  fs?: unknown;
  nodeWorkers?: number;
  packageManager?: unknown;
  threads?: number;
} & unknown;

export class AtlaspackV3 {
  _internal: AtlaspackNapi;

  constructor({
    fs,
    nodeWorkers,
    packageManager,
    threads,
    ...options
  }: AtlaspackV3Options) {
    // @ts-expect-error - TS2339 - Property 'logLevel' does not exist on type '{}'. | TS2339 - Property 'logLevel' does not exist on type '{}'.
    options.logLevel = options.logLevel || 'error';
    // @ts-expect-error - TS2339 - Property 'defaultTargetOptions' does not exist on type '{}'. | TS2339 - Property 'defaultTargetOptions' does not exist on type '{}'.
    options.defaultTargetOptions = options.defaultTargetOptions || {};
    // @ts-expect-error - TS2339 - Property 'defaultTargetOptions' does not exist on type '{}'. | TS2339 - Property 'defaultTargetOptions' does not exist on type '{}'.
    options.defaultTargetOptions.engines = options.defaultTargetOptions
      .engines || {
      browsers: [],
    };

    this._internal = new AtlaspackNapi({
      // @ts-expect-error - TS2322 - Type 'unknown' is not assignable to type 'object | undefined'.
      fs,
      nodeWorkers,
      // @ts-expect-error - TS2322 - Type 'unknown' is not assignable to type 'object | undefined'.
      packageManager,
      threads,
      options,
    });
  }

  async buildAssetGraph(): Promise<any> {
    const [workers, registerWorker] = this.#createWorkers();

    let result = await this._internal.buildAssetGraph({
      // @ts-expect-error - TS2322 - Type 'Worker[] | ((tx_worker: Transferable) => void)' is not assignable to type '(...args: any[]) => any'.
      registerWorker,
    });

    // @ts-expect-error - TS2488 - Type 'Worker[] | ((tx_worker: Transferable) => void)' must have a '[Symbol.iterator]()' method that returns an iterator.
    for (const worker of workers) worker.terminate();
    return result;
  }

  #createWorkers() {
    const workers: Array<Worker> = [];

    return [
      workers,
      (tx_worker: Transferable) => {
        let worker = new Worker(WORKER_PATH, {
          workerData: {
            tx_worker,
          },
        });
        workers.push(worker);
      },
    ];
  }
}
