// @flow
import assert from 'assert';
import * as napi from '@atlaspack/rust';
import {NodeFS} from '@atlaspack/fs';
import {NodePackageManager} from '@atlaspack/package-manager';
import type {
  Resolver,
  Transformer,
  PluginOptions,
  Dependency,
  FilePath,
  FileSystem,
  ConfigResultWithFilePath,
  PackageJSON,
} from '@atlaspack/types';
import {workerData} from 'worker_threads';
import * as module from 'module';

import {AssetCompat} from './compat';
import type {InnerAsset} from './compat';
import {jsCallable} from '../jsCallable';
import type {JsCallable} from '../jsCallable';
import Environment from '../../public/Environment';

const CONFIG = Symbol.for('parcel-plugin-config');

export class AtlaspackWorker {
  #resolvers: Map<string, ResolverState<any>>;
  #fs: FileSystem;

  constructor() {
    this.#resolvers = new Map();
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
      }
    },
  );

  runResolverResolve: JsCallable<
    [RunResolverResolveOptions],
    Promise<RunResolverResolveResult>,
  > = jsCallable(
    async ({key, dependency, specifier, pipeline, projectRoot}) => {
      const state = this.#resolvers.get(key);
      if (!state) {
        throw new Error(`Resolver not found: ${key}`);
      }

      let packageManager = state.packageManager;
      if (!packageManager) {
        packageManager = new NodePackageManager(this.#fs, projectRoot);
        state.packageManager = packageManager;
      }

      const defaultOptions = {
        get logger() {
          // $FlowFixMe
          return globalThis.console;
        },
        tracer: {
          enabled: false,
          createMeasurement: () => null,
        },
        options: {
          get mode() {
            throw new Error('Resolver.resolve.options.mode');
          },
          parcelVersion: 'TODO',
          packageManager,
          env: process.env,
          get hmrOptions() {
            throw new Error('Resolver.resolve.options.hmrOptions');
          },
          get serveOptions() {
            throw new Error('Resolver.resolve.options.serveOptions');
          },
          get shouldBuildLazily() {
            throw new Error('Resolver.resolve.options.shouldBuildLazily');
          },
          shouldAutoInstall: false,
          get logLevel() {
            throw new Error('Resolver.resolve.options.logLevel');
          },
          projectRoot,
          get cacheDir() {
            throw new Error('Resolver.resolve.options.cacheDir');
          },
          inputFS: this.#fs,
          outputFS: this.#fs,
          get instanceId() {
            throw new Error('Resolver.resolve.options.instanceId');
          },
          get detailedReport() {
            throw new Error('Resolver.resolve.options.detailedReport');
          },
          get featureFlags() {
            throw new Error('Resolver.resolve.options.featureFlags');
          },
        },
      };

      if (!('config' in state)) {
        state.config = await state.resolver.loadConfig?.({
          config: {
            isSource: true,
            searchPath: '',
            // $FlowFixMe This isn't correct to flow but is enough to satisfy the runtime checks
            env: new Environment(dependency.env, {
              projectRoot,
              hmrOptions: undefined,
            }),
            invalidateOnFileChange(): void {},
            invalidateOnFileCreate(): void {},
            invalidateOnEnvChange(): void {},
            invalidateOnStartup(): void {},
            invalidateOnBuild(): void {},
            addDevDependency(): void {},
            setCacheKey(): void {},
            getConfig<T>(): Promise<?ConfigResultWithFilePath<T>> {
              throw new Error('Resolver.loadConfig.config.getConfig');
            },
            getConfigFrom<T>(): Promise<?ConfigResultWithFilePath<T>> {
              throw new Error('Resolver.loadConfig.config.getConfigFrom');
            },
            getPackage(): Promise<?PackageJSON> {
              throw new Error('Resolver.loadConfig.config.getPackage');
            },
          },
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
          priority: result.priority && PriorityMap[result.priority],
          query: result.query && result.query.toString(),
        },
      };
    },
  );

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

type ResolverState<T> = {|
  resolver: Resolver<T>,
  config?: T,
  packageManager?: NodePackageManager,
|};

type LoadPluginOptions = {|
  kind: 'resolver',
  specifier: string,
  resolveFrom: string,
|};

type RunResolverResolveOptions = {|
  key: string,
  dependency: Dependency,
  specifier: FilePath,
  pipeline: ?string,
  projectRoot: string,
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
  conditional: 3,
};
