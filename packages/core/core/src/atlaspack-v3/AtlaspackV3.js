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
  _atlaspack_napi: AtlaspackNapi;
  _workerIds: any[];

  constructor(internal: AtlaspackNapi) {
    this._atlaspack_napi = internal;
  }

  static async new({
    fs,
    packageManager,
    threads,
    lmdb,
    napiWorkerPool = new NapiWorkerPool(),
    ...options
  }: AtlaspackV3Options): Promise<AtlaspackV3> {
    console.log('[start] AtlaspackV3.constructor');
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

    if (!internal) {
      throw new Error('What');
    }

    if (error !== null) {
      throw new ThrowableDiagnostic({
        diagnostic: error,
      });
    }

    const ap = new AtlaspackV3(internal);
    console.log('[end] AtlaspackV3.constructor', internal);
    return ap;
  }

  async buildAssetGraph(): Promise<any> {
    console.log('[start] buildAssetGraph', this._atlaspack_napi);
    let [graph, error] = await atlaspackNapiBuildAssetGraph(
      this._atlaspack_napi,
    );

    console.log('[end] buildAssetGraph', this._atlaspack_napi);
    if (error !== null) {
      throw new ThrowableDiagnostic({
        diagnostic: error,
      });
    }

    return graph;
  }

  async respondToFsEvents(events: Array<Event>): Promise<boolean> {
    log('[start] respondToFsEvents');
    let result = await atlaspackNapiRespondToFsEvents(
      this._atlaspack_napi,
      events,
    );
    log('[end] respondToFsEvents');

    return result;
  }

  async shutdown() {
    log('[start] shutdown');
    await atlaspackNapiShutdown(this._atlaspack_napi);
    log('[end] shutdown');
  }
}
