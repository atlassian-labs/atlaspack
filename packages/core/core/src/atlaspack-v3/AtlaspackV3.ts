import {
  atlaspackNapiCreate,
  atlaspackNapiBuildAssetGraph,
  atlaspackNapiRespondToFsEvents,
  AtlaspackNapi,
  Lmdb,
  AtlaspackNapiOptions,
} from '@atlaspack/rust';
import {NapiWorkerPool} from './NapiWorkerPool';
import ThrowableDiagnostic from '@atlaspack/diagnostic';
import type {Event} from '@parcel/watcher';
import type {NapiWorkerPool as INapiWorkerPool} from '@atlaspack/types';

export type AtlaspackV3Options = {
  fs?: AtlaspackNapiOptions['fs'];
  packageManager?: AtlaspackNapiOptions['packageManager'];
  threads?: number;
  /**
   * A reference to LMDB lite's rust object
   */
  lmdb: Lmdb;
  featureFlags?: {
    [key: string]: string | boolean;
  };
  napiWorkerPool?: INapiWorkerPool;
} & AtlaspackNapiOptions['options'];

export class AtlaspackV3 {
  _atlaspack_napi: AtlaspackNapi;
  _napiWorkerPool: INapiWorkerPool;
  _isDefaultNapiWorkerPool: boolean;

  constructor(
    atlaspack_napi: AtlaspackNapi,
    napiWorkerPool: INapiWorkerPool,
    isDefaultNapiWorkerPool: boolean,
  ) {
    this._atlaspack_napi = atlaspack_napi;
    this._napiWorkerPool = napiWorkerPool;
    this._isDefaultNapiWorkerPool = isDefaultNapiWorkerPool;
  }

  static async create({
    fs,
    packageManager,
    threads,
    lmdb,
    napiWorkerPool,
    ...options
  }: AtlaspackV3Options): Promise<AtlaspackV3> {
    options.logLevel = options.logLevel || 'error';
    options.defaultTargetOptions = options.defaultTargetOptions || {};
    options.defaultTargetOptions.engines =
      options.defaultTargetOptions.engines || {};

    let isDefaultNapiWorkerPool = false;
    if (!napiWorkerPool) {
      napiWorkerPool = new NapiWorkerPool();
      isDefaultNapiWorkerPool = true;
    }

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

    return new AtlaspackV3(internal, napiWorkerPool, isDefaultNapiWorkerPool);
  }

  end(): void {
    // If the worker pool was provided to us, don't shut it down, it's up to the provider.
    if (this._isDefaultNapiWorkerPool) {
      this._napiWorkerPool.shutdown();
    }
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
