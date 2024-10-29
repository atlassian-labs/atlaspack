// @flow

import {
  AtlaspackNapi,
  type Lmdb,
  type AtlaspackNapiOptions,
} from '@atlaspack/rust';
import {workerPool} from './WorkerPool';

export type AtlaspackV3Options = {|
  fs?: AtlaspackNapiOptions['fs'],
  nodeWorkers?: number,
  packageManager?: AtlaspackNapiOptions['packageManager'],
  threads?: number,
  /**
   * A reference to LMDB lite's rust object
   */
  lmdb: Lmdb | null,
  ...AtlaspackNapiOptions['options'],
|};

type WorkerMessage = {|type: 'workerRegistered'|} | {|type: 'pong'|};

class WorkerPool {
  workerPool: Worker[] = [];
  currentUsedWorkers: number = 0;

  waitMessage(worker: Worker, type: string): Promise<WorkerMessage> {
    return new Promise((resolve) => {
      const onMessage = (message: WorkerMessage) => {
        if (message.type === type) {
          resolve(message);
          worker.off('message', onMessage);
        }
      };
      worker.on('message', onMessage);
    });
  }

  async bootWorker(worker: Worker, tx_worker: number): Promise<void> {
    const timeout = new Promise((_, reject) => {
      setTimeout(() => {
        reject(new Error('Worker failed to register in time'));
      }, 2000);
    });
    const workerReady = this.waitMessage(worker, 'workerRegistered');
    worker.postMessage({type: 'registerWorker', tx_worker});
    await Promise.race([workerReady, timeout]);
  }

  registerWorker(tx_worker: number) {
    const workerIndex = this.currentUsedWorkers;
    let availableWorker = this.workerPool[workerIndex];
    if (availableWorker == null) {
      availableWorker = new Worker(WORKER_PATH, {});
      this.workerPool.push(availableWorker);
    }

    this.bootWorker(availableWorker, tx_worker).catch((err) => {
      // eslint-disable-next-line no-console
      console.error('Worker failed, retrying to create it...', err);
      this.workerPool[workerIndex] = new Worker(WORKER_PATH, {});
      this.bootWorker(this.workerPool[workerIndex], tx_worker);
    });

    this.currentUsedWorkers += 1;
  }

  reset() {
    this.currentUsedWorkers = 0;
  }
}

const workerPool = new WorkerPool();

export class AtlaspackV3 {
  _internal: AtlaspackNapi;

  constructor({
    fs,
    nodeWorkers,
    packageManager,
    threads,
    lmdb,
    ...options
  }: AtlaspackV3Options) {
    options.logLevel = options.logLevel || 'error';
    options.defaultTargetOptions = options.defaultTargetOptions || {};
    // $FlowFixMe "engines" are readonly
    options.defaultTargetOptions.engines = options.defaultTargetOptions
      .engines || {
      browsers: [],
    };

    this._internal = AtlaspackNapi.create(
      {
        fs,
        nodeWorkers,
        packageManager,
        threads,
        options,
      },
      lmdb,
    );
  }

  async buildAssetGraph(): Promise<any> {
    const workerIds = [];
    let result = await this._internal.buildAssetGraph({
      registerWorker: (tx_worker) => {
        // $FlowFixMe
        const workerId = workerPool.registerWorker(tx_worker);
        workerIds.push(workerId);
      },
    });

    workerPool.releaseWorkers(workerIds);

    return result;
  }
}
