// @flow
import assert from 'assert';
import * as napi from '@atlaspack/rust';
import type {
  Resolver,
  Transformer,
  PluginOptions,
  Dependency,
  FilePath,
} from '@atlaspack/types';
import {workerData} from 'worker_threads';
import * as module from 'module';

import {AssetCompat} from './compat';
import type {InnerAsset} from './compat';
import {jsCallable} from '../jsCallable';
import type {JsCallable} from '../jsCallable';

const CONFIG = Symbol.for('parcel-plugin-config');

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
          this.#resolvers.set(specifier, resolvedModule.default[CONFIG]);
          break;
      }
    },
  );

  runResolverResolve: JsCallable<
    [RunResolverResolveOptions],
    Promise<RunResolverResolveResult>,
  > = jsCallable(async ({key, dependency, specifier}) => {
    const resolver = this.#resolvers.get(key);
    if (!resolver) {
      throw new Error('Resolver not found');
    }

    const result = await resolver.resolve({
      specifier,
      dependency,
      get options() {
        throw new Error('TODO: Resolver.resolve.options');
      },
      get logger() {
        throw new Error('TODO: Resolver.resolve.logger');
      },
      get tracer() {
        throw new Error('TODO: Resolver.resolve.tracer');
      },
      get pipeline() {
        throw new Error('TODO: Resolver.resolve.pipeline');
      },
      get config() {
        throw new Error('TODO: Resolver.resolve.config');
      },
    });

    if (!result) {
      return {
        invalidations: [],
        resolution: {type: 'unresolved'},
      };
    }

    return {
      invalidations: [],
      resolution: {
        type: 'resolved',
        filePath: result.filePath || '',
        canDefer: result.canDefer || false,
        sideEffects: result.sideEffects || false,
        code: result.code || undefined,
        meta: result.meta || undefined,
        pipeline: result.pipeline || undefined,
        priority: result.priority && PriorityMap[result.priority],
        query: result.query && result.query.toString(),
      },
    };
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
    const customRequire = module.createRequire(resolveFrom);
    const resolvedPath = customRequire.resolve(specifier);
    // $FlowFixMe
    const transformerModule = await import(resolvedPath);
    const transformer: Transformer<*> =
      transformerModule.default.default[CONFIG];

    let assetCompat = new AssetCompat(asset, options);

    try {
      if (transformer.parse) {
        // $FlowFixMe
        const ast = await transformer.parse({asset: assetCompat}); // missing "config"
        // $FlowFixMe
        assetCompat.setAST(ast);
      }

      // $FlowFixMe
      const result = await transformer.transform({
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

type RunResolverResolveResult = {|
  invalidations: Array<*>,
  resolution:
    | {|type: 'unresolved'|}
    | {|type: 'excluded'|}
    | {|
        type: 'resolved',
        canDefer: boolean,
        filePath: string,
        sideEffects: boolean,
        code?: string,
        meta?: mixed,
        pipeline?: string,
        priority?: number,
        query?: string,
      |},
|};

const PriorityMap = {
  sync: 0,
  parallel: 1,
  lazy: 2,
};
