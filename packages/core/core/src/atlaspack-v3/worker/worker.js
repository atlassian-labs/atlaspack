// @flow
import assert from 'assert';
import * as napi from '@atlaspack/rust';
import type {JsCallable} from '@atlaspack/rust';
import {NodeFS} from '@atlaspack/fs';
import {NodePackageManager} from '@atlaspack/package-manager';
import type {
  Resolver,
  Transformer,
  FilePath,
  FileSystem,
} from '@atlaspack/types';
import {parentPort} from 'worker_threads';
import * as module from 'module';

import {jsCallable} from '../jsCallable';
import {
  Environment,
  Dependency,
  PluginConfig,
  PluginLogger,
  PluginTracer,
  PluginOptions,
  MutableAsset,
  bundleBehaviorMap,
  dependencyPriorityMap,
} from './compat';

const CONFIG = Symbol.for('parcel-plugin-config');

export class AtlaspackWorker {
  #resolvers: Map<string, ResolverState<any>>;
  #transformers: Map<string, TransformerState<any>>;
  #fs: FileSystem;

  constructor() {
    this.#resolvers = new Map();
    this.#transformers = new Map();
    this.#fs = new NodeFS();
  }

  loadPlugin: JsCallable<[LoadPluginOptions], Promise<void>> = jsCallable(
    async ({kind, specifier, resolveFrom}) => {
      let customRequire = module.createRequire(resolveFrom);
      let resolvedPath = customRequire.resolve(specifier);
      // $FlowFixMe
      let resolvedModule = await import(resolvedPath);

      let instance = undefined;
      if (resolvedModule.default && resolvedModule.default[CONFIG]) {
        instance = resolvedModule.default[CONFIG];
      } else if (
        resolvedModule.default &&
        resolvedModule.default.default &&
        resolvedModule.default.default[CONFIG]
      ) {
        instance = resolvedModule.default.default[CONFIG];
      } else {
        throw new Error(
          `Plugin could not be resolved\n\t${kind}\n\t${resolveFrom}\n\t${specifier}`,
        );
      }

      switch (kind) {
        case 'resolver':
          this.#resolvers.set(specifier, {resolver: instance});
          break;
        case 'transformer':
          this.#transformers.set(specifier, {transformer: instance});
          break;
      }
    },
  );

  runResolverResolve: JsCallable<
    [RunResolverResolveOptions],
    Promise<RunResolverResolveResult>,
  > = jsCallable(
    async ({
      key,
      dependency: napiDependency,
      specifier,
      pipeline,
      pluginOptions,
    }) => {
      const state = this.#resolvers.get(key);
      if (!state) {
        throw new Error(`Resolver not found: ${key}`);
      }

      let packageManager = state.packageManager;
      if (!packageManager) {
        packageManager = new NodePackageManager(
          this.#fs,
          pluginOptions.projectRoot,
        );
        state.packageManager = packageManager;
      }

      const env = new Environment(napiDependency.env);
      const dependency = new Dependency(napiDependency, env);

      const defaultOptions = {
        logger: new PluginLogger(),
        tracer: new PluginTracer(),
        options: new PluginOptions({
          ...pluginOptions,
          packageManager,
          shouldAutoInstall: false,
          inputFS: this.#fs,
          outputFS: this.#fs,
        }),
      };

      if (!('config' in state)) {
        state.config = await state.resolver.loadConfig?.({
          config: new PluginConfig({
            env,
            isSource: true,
            searchPath: specifier,
            projectRoot: pluginOptions.projectRoot,
            fs: this.#fs,
            packageManager,
          }),
          ...defaultOptions,
        });
      }

      const result = await state.resolver.resolve({
        specifier,
        dependency,
        pipeline,
        config: state.config,
        ...defaultOptions,
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
          priority: dependencyPriorityMap.intoNullable(result.priority),
          query: result.query && result.query.toString(),
        },
      };
    },
  );

  runTransformerTransform: JsCallable<
    [RunTransformerTransformOptions, Buffer, ?string],
    Promise<RunTransformerTransformResult>,
  > = jsCallable(
    async ({key, env: napiEnv, options, asset: innerAsset}, contents, map) => {
      const state = this.#transformers.get(key);
      if (!state) {
        throw new Error(`Transformer not found: ${key}`);
      }

      let packageManager = state.packageManager;
      if (!packageManager) {
        packageManager = new NodePackageManager(this.#fs, options.projectRoot);
        state.packageManager = packageManager;
      }

      const transformer: Transformer<any> = state.transformer;
      const resolveFunc = (from: string, to: string): Promise<any> => {
        let customRequire = module.createRequire(from);
        let resolvedPath = customRequire.resolve(to);

        return Promise.resolve(resolvedPath);
      };

      const env = new Environment(napiEnv);
      const mutableAsset = new MutableAsset(
        innerAsset,
        contents,
        env,
        this.#fs,
        map,
        options.projectRoot,
      );

      const defaultOptions = {
        logger: new PluginLogger(),
        tracer: new PluginTracer(),
        options: new PluginOptions({
          ...options,
          packageManager,
          shouldAutoInstall: false,
          inputFS: this.#fs,
          outputFS: this.#fs,
        }),
      };

      const config = await transformer.loadConfig?.({
        config: new PluginConfig({
          env,
          isSource: true,
          searchPath: innerAsset.filePath,
          projectRoot: options.projectRoot,
          fs: this.#fs,
          packageManager,
        }),
        ...defaultOptions,
      });

      if (transformer.parse) {
        const ast = await transformer.parse({
          asset: mutableAsset,
          config,
          resolve: resolveFunc,
          ...defaultOptions,
        });
        if (ast) {
          mutableAsset.setAST(ast);
        }
      }

      const result = await state.transformer.transform({
        asset: mutableAsset,
        config,
        resolve: resolveFunc,
        ...defaultOptions,
      });

      if (transformer.generate) {
        const ast = await mutableAsset.getAST();
        if (ast) {
          // $FlowFixMe "Cannot call `transformer.generate` because  undefined [1] is not a function." 🤷‍♀️
          const output = await transformer.generate({
            asset: mutableAsset,
            ast,
            ...defaultOptions,
          });

          if (typeof output.content === 'string') {
            mutableAsset.setCode(output.content);
          } else if (output.content instanceof Buffer) {
            mutableAsset.setBuffer(output.content);
          } else {
            mutableAsset.setStream(output.content);
          }

          if (output.map) {
            mutableAsset.setMap(output.map);
          }
        }
      }

      assert(
        result.length === 1,
        '[V3] Unimplemented: Multiple asset return from Node transformer',
      );

      assert(
        result[0] === mutableAsset,
        '[V3] Unimplemented: New asset returned from Node transformer',
      );

      return [
        {
          id: mutableAsset.id,
          bundleBehavior: bundleBehaviorMap.intoNullable(
            mutableAsset.bundleBehavior,
          ),
          code: [],
          filePath: mutableAsset.filePath,
          isBundleSplittable: mutableAsset.isBundleSplittable,
          isSource: mutableAsset.isSource,
          meta: mutableAsset.meta,
          pipeline: mutableAsset.pipeline,
          // Query should be undefined if it's empty
          query: mutableAsset.query.toString() || undefined,
          sideEffects: mutableAsset.sideEffects,
          symbols: mutableAsset.symbols.intoNapi(),
          type: mutableAsset.type,
          uniqueKey: mutableAsset.uniqueKey,
        },
        await mutableAsset.getBuffer(),
        // Only send back the map if it has changed
        mutableAsset.isMapDirty
          ? // $FlowFixMe A dirty map means this will be valid
            JSON.stringify((await mutableAsset.getMap()).toVLQ())
          : '',
      ];
    },
  );
}

// Create napi worker and send it back to main thread
const worker = new AtlaspackWorker();
const napiWorker = napi.newNodejsWorker(worker);
parentPort?.postMessage(napiWorker);

type ResolverState<T> = {|
  resolver: Resolver<T>,
  config?: T,
  packageManager?: NodePackageManager,
|};

type TransformerState<T> = {|
  packageManager?: NodePackageManager,
  transformer: Transformer<T>,
|};

type LoadPluginOptions = {|
  kind: 'resolver' | 'transformer',
  specifier: string,
  resolveFrom: string,
|};

type RpcPluginOptions = {|
  projectRoot: string,
  mode: string,
|};

type RunResolverResolveOptions = {|
  key: string,
  dependency: napi.Dependency,
  specifier: FilePath,
  pipeline: ?string,
  pluginOptions: RpcPluginOptions,
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
        priority?: ?number,
        query?: string,
      |},
|};

type RunTransformerTransformOptions = {|
  key: string,
  env: napi.Environment,
  options: RpcPluginOptions,
  asset: napi.Asset,
|};

type RunTransformerTransformResult = [napi.RpcAssetResult, Buffer, string];
