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
    // @ts-expect-error - TS1064 - The return type of an async function or method must be the global Promise<T> type. Did you mean to write 'Promise<any>'?
  }): any {
    // @ts-expect-error - TS2339 - Property 'createRequire' does not exist on type 'typeof Module'.
    const customRequire = module.createRequire(resolveFrom);
    const resolvedPath = customRequire.resolve(specifier);
    const transformerModule = await import(resolvedPath);
    const transformer: Transformer<any> =
      transformerModule.default.default[CONFIG];

    let assetCompat = new AssetCompat(asset, options);

    try {
      if (transformer.parse) {
        // @ts-expect-error - TS2740 - Type 'AssetCompat' is missing the following properties from type 'Asset': stats, fs, query, env, and 13 more.
        const ast = await transformer.parse({asset: assetCompat}); // missing "config"
        // @ts-expect-error - TS2345 - Argument of type 'AST | null | undefined' is not assignable to parameter of type 'AST'.
        assetCompat.setAST(ast);
      }

      const result = await transformer.transform({
        // $FlowFixMe
        // @ts-expect-error - TS2740 - Type 'AssetCompat' is missing the following properties from type 'MutableAsset': isBundleSplittable, sideEffects, uniqueKey, symbols, and 22 more.
        asset: assetCompat,
        options,
        config: null,
      });

      if (transformer.generate) {
        let output = await transformer.generate({
          // $FlowFixMe
          // @ts-expect-error - TS2322 - Type 'AssetCompat' is not assignable to type 'Asset'.
          asset: assetCompat,
          // $FlowFixMe
          // @ts-expect-error - TS2322 - Type 'AST | null | undefined' is not assignable to type 'AST'.
          ast: assetCompat.getAST(),
        });
        // @ts-expect-error - TS2345 - Argument of type 'Blob' is not assignable to parameter of type 'string'.
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
