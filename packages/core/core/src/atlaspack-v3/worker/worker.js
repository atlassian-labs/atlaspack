// @flow
import assert from 'assert';
import * as napi from '@atlaspack/rust';
import type {
  Resolver,
  Transformer,
  PluginOptions,
  Dependency,
  FilePath,
  ResolveResult,
} from '@atlaspack/types';
import {workerData} from 'worker_threads';
import * as module from 'module';

import {AssetCompat} from './compat';
import type {InnerAsset} from './compat';
import {jsCallable} from '../jsCallable';
import type {JsCallable} from '../jsCallable';

const CONFIG = Symbol.for('parcel-plugin-config');

type LoadPluginOptions = {|
  kind: 'resolver',
  specifier: string,
  resolveFrom: string,
|};

type RunResolverResolveOptions = {|
  key: string,
  dependency: Dependency,
  specifier: FilePath,
|};

export class AtlaspackWorker {
  #resolvers: Map<string, Resolver<*>>;

  constructor() {
    this.#resolvers = new Map();
  }

  loadPlugin: JsCallable<[LoadPluginOptions], Promise<void>> = jsCallable(
    async ({kind, specifier, resolveFrom}) => {
      let customRequire = module.createRequire(resolveFrom);
      let resolvedPath = customRequire.resolve(specifier);
      // $FlowFixMe
      let resolvedModule = await import(resolvedPath);

      switch (kind) {
        case 'resolver':
          this.#resolvers.set(specifier, resolvedModule);
          break;
      }
    },
  );

  runResolverResolve: JsCallable<
    [RunResolverResolveOptions],
    Promise<?ResolveResult>,
  > = jsCallable(async ({key, dependency, specifier}) => {
    const resolver = this.#resolvers.get(key);
    if (!resolver) {
      throw new Error('Resolver not found');
    }

    const result = await resolver.resolve({
      specifier,
      dependency,
      get options() {
        throw new Error('TODO: ResolverArgs.options');
      },
      get logger() {
        throw new Error('TODO: ResolverArgs.options');
      },
      get tracer() {
        throw new Error('TODO: ResolverArgs.options');
      },
      get pipeline() {
        throw new Error('TODO: ResolverArgs.options');
      },
      get config() {
        throw new Error('TODO: ResolverArgs.options');
      },
    });

    return result;
  });

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

    try {
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
    } catch (e) {
      // TODO: Improve error logging from JS plugins. Without this you currently
      // only see the error message, no stack trace.
      // eslint-disable-next-line no-console
      console.error(e);
      throw e;
    }
  }
}

napi.registerWorker(workerData.tx_worker, new AtlaspackWorker());
