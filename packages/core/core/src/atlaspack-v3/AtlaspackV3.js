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
import {isSuperPackage} from '../isSuperPackage';
import path from 'path';

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
  ...$Diff<
    AtlaspackNapiOptions['options'],
    {|
      jsPaths: AtlaspackNapiOptions['options']['jsPaths'],
    |},
  >,
|};

function getJsPaths(): AtlaspackNapiOptions['options']['jsPaths'] {
  const dirname = /*#__ATLASPACK_IGNORE__*/ __dirname;

  if (isSuperPackage()) {
    // dirname: atlaspack/lib/core/core/atlaspack-v3
    // core: atlaspack/lib/core/core/index.js
    const corePath = path.join(dirname, '..');
    // esmodule helpers: atlaspack/lib/transformers/js/esmodule-helpers.js
    const esmoduleHelpersPath = path.join(
      dirname,
      '../../../transformers/js/esmodule-helpers.js',
    );

    // empty file: atlaspack/lib/core/core/_empty.js
    const emptyFile = path.join(dirname, '_empty.js');

    return {
      corePath,
      esmoduleHelpersSpecifier: path.relative(corePath, esmoduleHelpersPath),
      esmoduleHelpersIncludeNodeModules: 'atlaspack',
      emptyFile,
    };
  }

  // dirname: @atlaspack/core/lib/atlaspack-v3
  // core: @atlaspack/core
  const corePath = path.join(dirname, '../..');
  // empty file: atlaspack/lib/core/core/_empty.js
  const emptyFile = path.join(dirname, '_empty.js');

  return {
    corePath,
    esmoduleHelpersSpecifier:
      '@atlaspack/transformer-js/src/esmodule-helpers.js',
    esmoduleHelpersIncludeNodeModules: '@atlaspack/transformer-js',
    emptyFile,
  };
}

export class AtlaspackV3 {
  _atlaspack_napi: AtlaspackNapi;

  constructor(atlaspack_napi: AtlaspackNapi) {
    this._atlaspack_napi = atlaspack_napi;
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
        options: {
          ...options,
          jsPaths: getJsPaths(),
        },
        napiWorkerPool,
      },
      lmdb,
    );

    if (error !== null) {
      throw new ThrowableDiagnostic({
        diagnostic: error,
      });
    }

    return new AtlaspackV3(internal);
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
