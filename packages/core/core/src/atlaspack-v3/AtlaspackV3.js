// @flow

import {
  AtlaspackNapi,
  type Lmdb,
  type AtlaspackNapiOptions,
} from '@atlaspack/rust';
import {WorkerPoolV3} from './WorkerPoolV3';
import ThrowableDiagnostic from '@atlaspack/diagnostic';
import type {Event} from '@parcel/watcher';
import type {WorkerPoolV3 as IWorkerPoolV3} from '@atlaspack/types';

export type AtlaspackV3Options = {|
  fs?: AtlaspackNapiOptions['fs'],
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

    this._internal = AtlaspackNapi.create(
      {
        fs,
        packageManager,
        threads,
        options,
        workerPoolV3,
      },
      lmdb,
    );
  }

  async buildAssetGraph(): Promise<any> {
    let [graph, error] = await this._internal.buildAssetGraph();

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
