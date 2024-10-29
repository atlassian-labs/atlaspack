// @flow

import path from 'path';
import {Worker} from 'worker_threads';
import {AtlaspackNapi, type AtlaspackNapiOptions} from '@atlaspack/rust';

const WORKER_PATH = path.join(__dirname, 'worker', 'index.js');

export type AtlaspackV3Options = {|
  fs?: AtlaspackNapiOptions['fs'],
  nodeWorkers?: number,
  packageManager?: AtlaspackNapiOptions['packageManager'],
  threads?: number,
  ...AtlaspackNapiOptions['options'],
|};

class WorkerPool {
  workerPool: Worker[] = [];
  currentUsedWorkers: number = 0;

  registerWorker(tx_worker) {
    let availableWorker = this.workerPool[this.currentUsedWorkers];
    if (availableWorker == null) {
      availableWorker = new Worker(WORKER_PATH, {
        workerData: {
          tx_worker,
        },
      });
      this.workerPool.push(availableWorker);
    } else {
      availableWorker.postMessage({
        type: 'registerWorker',
        tx_worker,
      });
    }

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
    ...options
  }: AtlaspackV3Options) {
    options.logLevel = options.logLevel || 'error';
    options.defaultTargetOptions = options.defaultTargetOptions || {};
    // $FlowFixMe "engines" are readonly
    options.defaultTargetOptions.engines = options.defaultTargetOptions
      .engines || {
      browsers: [],
    };

    this._internal = new AtlaspackNapi({
      fs,
      nodeWorkers,
      packageManager,
      threads,
      options,
    });
  }

  async buildAssetGraph(): Promise<any> {
    let result = await this._internal.buildAssetGraph({
      registerWorker: (tx_worker) => workerPool.registerWorker(tx_worker),
    });

    workerPool.reset();

    return result;
  }
}
