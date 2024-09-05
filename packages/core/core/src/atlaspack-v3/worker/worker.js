// @flow
import assert from 'assert';
import * as napi from '@atlaspack/rust';
import type {Transformer, PluginOptions} from '@atlaspack/types';
import {workerData} from 'worker_threads';
import * as module from 'module';

import {AssetCompat} from './compat';
import type {InnerAsset} from './compat';
import {transform} from '@babel/core';

const CONFIG = Symbol.for('parcel-plugin-config');

export class AtlaspackWorker {
  ping() {
    // console.log('Hi');
  }

  async runTransformer({
    resolveFrom,
    specifier,
    options,
    asset,
  }: {|
    resolveFrom: string,
    specifier: string,
    options: PluginOptions,
    asset: InnerAsset,
  |}): any {
    let customRequire = module.createRequire(resolveFrom);
    let resolvedPath = customRequire.resolve(specifier);
    // $FlowFixMe
    let transformerModule = await import(resolvedPath);
    let transformer: Transformer<*> = transformerModule.default.default[CONFIG];

    let assetCompat = new AssetCompat(asset, options);

    if (transformer.parse) {
      // $FlowFixMe
      let ast = await transformer.parse({asset: assetCompat});
      // $FlowFixMe
      assetCompat.setAST(ast);
    }

    // $FlowFixMe
    let result = await transformer.transform({
      // $FlowFixMe
      asset: assetCompat,
      options,
      config: null,
    });

    if (transformer.generate) {
      // $FlowFixMe
      let output = await transformer.generate({
        // $FlowFixMe
        asset: assetCompat,
        // $FlowFixMe
        ast: assetCompat.getAST(),
      });
      // $FlowFixMe
      assetCompat.setCode(output.content);
    }

    assert(
      result.length === 1,
      '[V3] Unimplemented: Multiple asset return from Node transformer',
    );
    assert(
      result[0] === assetCompat,
      '[V3] Unimplemented: New asset returned from Node transformer',
    );

    return {
      asset,
      dependencies: assetCompat._dependencies,
    };
  }
}

napi.registerWorker(workerData.tx_worker, new AtlaspackWorker());
