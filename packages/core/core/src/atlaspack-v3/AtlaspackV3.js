// @flow

import {workerPool} from './WorkerPool';
import {AtlaspackNapi, type AtlaspackNapiOptions} from '@atlaspack/rust';

export type AtlaspackV3Options = {|
  fs?: AtlaspackNapiOptions['fs'],
  nodeWorkers?: number,
  packageManager?: AtlaspackNapiOptions['packageManager'],
  threads?: number,
  ...AtlaspackNapiOptions['options'],
|};

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
