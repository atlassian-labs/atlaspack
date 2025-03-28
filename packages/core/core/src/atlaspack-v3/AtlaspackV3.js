// @flow

import {
  AtlaspackNapi,
  atlaspackNapiCreate,
  atlaspackNapiBuildAssetGraph,
  atlaspackNapiRespondToFsEvents,
  atlaspackNapiShutdown,
  type Lmdb,
  type AtlaspackNapiOptions,
} from '@atlaspack/rust';
import {NapiWorkerPool} from './NapiWorkerPool';
import ThrowableDiagnostic from '@atlaspack/diagnostic';
import type {Event} from '@parcel/watcher';
import type {NapiWorkerPool as INapiWorkerPool} from '@atlaspack/types';

export type AtlaspackV3Options = {|
  fs?: AtlaspackNapiOptions['fs'],
  packageManager?: AtlaspackNapiOptions['packageManager'],
  threads?: number,
  /**
   * A reference to LMDB lite's rust object
   */
  lmdb: Lmdb,
  featureFlags?: {[string]: string | boolean},
  napiWorkerPool?: INapiWorkerPool,
  ...AtlaspackNapiOptions['options'],
|};

function log(msg) {
  if (process.env.LOG) {
    console.log(msg);
  }
}
export class AtlaspackV3 {
  _internal: AtlaspackNapi;
  _workerIds: any[];

  constructor(internal: AtlaspackNapi) {
    this._internal = internal;
  }

  static async new({
    fs,
    packageManager,
    threads,
    lmdb,
    napiWorkerPool = new NapiWorkerPool(),
    ...options
  }: AtlaspackV3Options): Promise<AtlaspackV3> {
    log('[start] AtlaspackV3.constructor');
    options.logLevel = options.logLevel || 'error';
    options.defaultTargetOptions = options.defaultTargetOptions || {};
    // $FlowFixMe "engines" are readonly
    options.defaultTargetOptions.engines =
      options.defaultTargetOptions.engines || {};

    const [internal, error] = await atlaspackNapiCreate(
      {
        fs,
        packageManager,
        threads,
        options,
        napiWorkerPool,
      },
      lmdb,
    );
    if (error !== null) {
      throw new ThrowableDiagnostic({
        diagnostic: error,
      });
    }
    log('[end] AtlaspackV3.constructor');
    return new AtlaspackV3(internal);
  }

  async buildAssetGraph(): Promise<any> {
    log('[start] buildAssetGraph');

    let [graph, error] = await atlaspackNapiBuildAssetGraph(this._internal);

    log('[end] buildAssetGraph');
    if (error !== null) {
      throw new ThrowableDiagnostic({
        diagnostic: error,
      });
    }

    return graph;
  }

  async respondToFsEvents(events: Array<Event>): Promise<boolean> {
    log('[start] respondToFsEvents');
    let result = await atlaspackNapiRespondToFsEvents(this._internal, events);
    log('[end] respondToFsEvents');

    return result;
  }

  async shutdown() {
    log('[start] shutdown');
    await atlaspackNapiShutdown(this._internal);
    log('[end] shutdown');
  }
}
