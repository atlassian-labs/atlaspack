// @flow

import {
  AtlaspackNapi,
  type Lmdb,
  type AtlaspackNapiOptions,
} from '@atlaspack/rust';
import {workerPool} from './WorkerPool';
import ThrowableDiagnostic from '@atlaspack/diagnostic';

export type AtlaspackV3Options = {|
  fs?: AtlaspackNapiOptions['fs'],
  nodeWorkers?: number,
  packageManager?: AtlaspackNapiOptions['packageManager'],
  threads?: number,
  /**
   * A reference to LMDB lite's rust object
   */
  lmdb: Lmdb,
  ...AtlaspackNapiOptions['options'],
|};

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
    options.defaultTargetOptions.engines =
      options.defaultTargetOptions.engines || {};

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

    workerPool.releaseWorkers(workerIds);

    if (error !== null) {
      throw new ThrowableDiagnostic({
        diagnostic: error,
      });
    }

    return graph;
  }
}
