// @flow
import * as napi from '@atlaspack/rust';
import type {Transformer} from '@atlaspack/types';
import {workerData} from 'worker_threads';
import * as module from 'module';

export class AtlaspackWorker {
  #transformers: Map<string, Transformer<any>>;

  constructor() {
    this.#transformers = new Map();
  }

  ping() {
    // console.log('Hi');
  }

  transformTransformer(key: string, asset: any) {
    console.log({key, asset});
  }

  async registerTransformer(resolve_from: string, specifier: string) {
    let customRequire = module.createRequire(resolve_from);
    let resolvedPath = customRequire.resolve(specifier);
    // $FlowFixMe
    let transformer = await import(resolvedPath);
    this.#transformers.set(`${resolve_from}:${specifier}`, transformer);
  }
}

napi.registerWorker(workerData.tx_worker, new AtlaspackWorker());
