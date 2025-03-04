// @flow

import {
  AtlaspackNapi,
  type Lmdb,
  type AtlaspackNapiOptions,
} from '@atlaspack/rust';
import {workerPool} from './WorkerPool';
import {WorkerPoolV3} from './WorkerPoolV3';
import ThrowableDiagnostic from '@atlaspack/diagnostic';
import type {Event} from '@parcel/watcher';
import type {WorkerPoolV3 as IWorkerPoolV3} from '@atlaspack/types';

export type AtlaspackV3Options = {|
  fs?: AtlaspackNapiOptions['fs'],
  nodeWorkers?: number,
  packageManager?: AtlaspackNapiOptions['packageManager'],
  threads?: number,
  /**
   * A reference to LMDB lite's rust object
   */
  lmdb: Lmdb,
  featureFlags?: {[string]: string | boolean},
  workerPoolV3?: IWorkerPoolV3,
  ...AtlaspackNapiOptions['options'],
|};

export class AtlaspackV3 {
  _internal: AtlaspackNapi;
  _workerIds: any[];

  constructor({
    fs,
    nodeWorkers,
    packageManager,
    threads,
    lmdb,
    workerPoolV3 = new WorkerPoolV3(),
    ...options
  }: AtlaspackV3Options) {
    options.logLevel = options.logLevel || 'error';
    options.defaultTargetOptions = options.defaultTargetOptions || {};
    // $FlowFixMe "engines" are readonly
    options.defaultTargetOptions.engines =
      options.defaultTargetOptions.engines || {};

    console.log(workerPoolV3);

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

    let [graph, error] = await this._internal.buildAssetGraph({
      registerWorker: (tx_worker) => {
        // $FlowFixMe
        const workerId = workerPool.registerWorker(tx_worker);
        workerIds.push(workerId);
      },
    });

    // In the integration tests we keep the workers alive so they don't need to
    // be re-initialized for the next test
    if (process.env.ATLASPACK_BUILD_ENV === 'test') {
      workerPool.releaseWorkers(workerIds);
    } else {
      workerPool.shutdown();
    }

    if (error !== null) {
      throw new ThrowableDiagnostic({
        diagnostic: error,
      });
    }

    return graph;
  }

  respondToFsEvents(events: Array<Event>): boolean {
    return this._internal.respondToFsEvents(events);
  }
}
