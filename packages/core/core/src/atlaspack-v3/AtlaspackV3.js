// @flow

import {
  atlaspackNapiCreate,
  atlaspackNapiBuildAssetGraph,
  atlaspackNapiRespondToFsEvents,
  type AtlaspackNapi,
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

export class AtlaspackV3 {
  _atlaspack_napi: AtlaspackNapi;
  _napiWorkerPool: INapiWorkerPool;

  constructor(atlaspack_napi: AtlaspackNapi, napiWorkerPool: INapiWorkerPool) {
    this._atlaspack_napi = atlaspack_napi;
    this._napiWorkerPool = napiWorkerPool;
  }

  static async create({
    fs,
    packageManager,
    threads,
    lmdb,
    napiWorkerPool = new NapiWorkerPool(),
    ...options
  }: AtlaspackV3Options): Promise<AtlaspackV3> {
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

    return new AtlaspackV3(internal, napiWorkerPool);
  }

  end(): void {
    this._napiWorkerPool.shutdown();
  }

  async buildAssetGraph(): Promise<any> {
    let [graph, error] = await atlaspackNapiBuildAssetGraph(
      this._atlaspack_napi,
    );

    if (error !== null) {
      throw new ThrowableDiagnostic({
        diagnostic: error,
      });
    }

    return graph;
  }

  respondToFsEvents(events: Array<Event>): boolean {
    return atlaspackNapiRespondToFsEvents(this._atlaspack_napi, events);
  }
}
