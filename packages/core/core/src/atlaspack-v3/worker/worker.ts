import assert from 'assert';
import * as napi from '@atlaspack/rust';
import type {Transformer, PluginOptions} from '@atlaspack/types';
import {workerData} from 'worker_threads';
import * as module from 'module';

import {AssetCompat} from './compat';
import type {InnerAsset} from './compat';

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
  }: {
    resolveFrom: string;
    specifier: string;
    options: PluginOptions;
    asset: InnerAsset;
  }): any {
    const customRequire = module.createRequire(resolveFrom);
    const resolvedPath = customRequire.resolve(specifier);
    const transformerModule = await import(resolvedPath);
    const transformer: Transformer<any> =
      transformerModule.default.default[CONFIG];

    let assetCompat = new AssetCompat(asset, options);

    try {
      if (transformer.parse) {
        const ast = await transformer.parse({asset: assetCompat}); // missing "config"
        assetCompat.setAST(ast);
      }

      const result = await transformer.transform({
        // $FlowFixMe
        asset: assetCompat,
        options,
        config: null,
      });

      if (transformer.generate) {
        let output = await transformer.generate({
          // $FlowFixMe
          asset: assetCompat,
          // $FlowFixMe
          ast: assetCompat.getAST(),
        });
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
    } catch (e: any) {
      // TODO: Improve error logging from JS plugins. Without this you currently
      // only see the error message, no stack trace.
      // eslint-disable-next-line no-console
      console.error(e);
      throw e;
    }
  }
}

napi.registerWorker(workerData.tx_worker, new AtlaspackWorker());
