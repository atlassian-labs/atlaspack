/* eslint-disable no-console */
/* eslint-disable no-unused-vars */
// @flow
import * as napi from '@atlaspack/rust';
import type {Transformer} from '@atlaspack/types';
import {workerData} from 'worker_threads';
import * as module from 'module';
import type {NapiTransformResult} from '../plugins';
import {MappedNapiAsset} from '../plugins';

const CONFIG = Symbol.for('parcel-plugin-config');

export class AtlaspackWorker {
  #transformers: Map<string, Transformer<any>>;

  constructor() {
    this.#transformers = new Map();
  }

  ping() {
    // console.log('Hi');
  }

  async transformTransformer(
    key: string,
    asset: any,
  ): Promise<NapiTransformResult> {
    let transformer = this.#transformers.get(key);

    let result: NapiTransformResult = {
      asset,
      dependencies: [],
      invalidateOnFileChange: [],
    };

    let transformResult = await transformer?.transform({
      asset: new MappedNapiAsset(asset, result.dependencies),
      // $FlowFixMe
      config: {},
      // $FlowFixMe
      resolve: {},
      // $FlowFixMe
      options: {},
      // $FlowFixMe
      logger: {},
      // $FlowFixMe
      tracer: {},
    });

    // console.log(result);
    return result;
  }

  async registerTransformer(resolve_from: string, specifier: string) {
    let customRequire = module.createRequire(resolve_from);
    let resolvedPath = customRequire.resolve(specifier);
    // $FlowFixMe
    let transformerModule = await import(resolvedPath);
    let transformer = transformerModule.default;
    let transformerOpts = transformer[CONFIG];

    this.#transformers.set(`${resolve_from}:${specifier}`, transformerOpts);
  }
}

napi.registerWorker(workerData.tx_worker, new AtlaspackWorker());
