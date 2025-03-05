// @flow

import {
  AtlaspackNapi,
  type Lmdb,
  type AtlaspackNapiOptions,
} from '@atlaspack/rust';
import {workerPool} from './WorkerPool';
import ThrowableDiagnostic from '@atlaspack/diagnostic';
import type {Event} from '@parcel/watcher';

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
    ...options
  }: AtlaspackV3Options) {
    options.logLevel = options.logLevel || 'error';
    options.defaultTargetOptions = options.defaultTargetOptions || {};
    // $FlowFixMe "engines" are readonly
    options.defaultTargetOptions.engines =
      options.defaultTargetOptions.engines || {};

    this._workerIds = [];
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
