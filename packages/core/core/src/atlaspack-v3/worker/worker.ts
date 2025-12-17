/* eslint-disable import/first */
import {SideEffectDetector} from './side-effect-detector';

// Install side effect detection patches BEFORE importing any modules that use fs
const sideEffectDetector = new SideEffectDetector();
sideEffectDetector.install();

import assert from 'assert';
import * as napi from '@atlaspack/rust';
// @ts-expect-error TS2305
import type {JsCallable} from '@atlaspack/rust';
import {NodeFS} from '@atlaspack/fs';
import {NodePackageManager} from '@atlaspack/package-manager';
import type {
  Resolver,
  Transformer,
  FilePath,
  FileSystem,
} from '@atlaspack/types';
import type {FeatureFlags} from '@atlaspack/feature-flags';
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
  #packageManager: NodePackageManager;
  #options: Options | undefined;
  #sideEffectDetector: SideEffectDetector;

  constructor() {
    this.#resolvers = new Map();
    this.#transformers = new Map();
    this.#fs = new NodeFS();
    this.#packageManager = new NodePackageManager(this.#fs, '/');
    this.#sideEffectDetector = sideEffectDetector; // Use the global detector that was installed before imports
  }

  clearState() {
    this.#resolvers.clear();
    this.#transformers.clear();
    this.#options = undefined;
  }

  loadPlugin: JsCallable<[LoadPluginOptions], Promise<undefined>> = jsCallable(
    async ({kind, specifier, resolveFrom, options}) => {
      // Use packageManager.require() instead of dynamic import() to support TypeScript plugins
      let resolvedModule = await this.#packageManager.require(
        specifier,
        resolveFrom,
        {shouldAutoInstall: false},
      );

      let instance = undefined;
      // Check for CommonJS export (module.exports = new Plugin(...))
      if (resolvedModule[CONFIG]) {
        instance = resolvedModule[CONFIG];
      } else if (resolvedModule.default && resolvedModule.default[CONFIG]) {
        // ESM default export
        instance = resolvedModule.default[CONFIG];
      } else if (
        resolvedModule.default &&
        resolvedModule.default.default &&
        resolvedModule.default.default[CONFIG]
      ) {
        // Double-wrapped default export
        instance = resolvedModule.default.default[CONFIG];
      } else {
        throw new Error(
          `Plugin could not be resolved\n\t${kind}\n\t${resolveFrom}\n\t${specifier}`,
        );
      }

      if (this.#options == null) {
        this.#options = {
          ...options,
          inputFS: this.#fs,
          outputFS: this.#fs,
          packageManager: this.#packageManager,
          shouldAutoInstall: false,
        };
      }

      // Set feature flags in the worker process
      let featureFlagsModule = await this.#packageManager.require(
        '@atlaspack/feature-flags',
        __filename,
        {shouldAutoInstall: false},
      );
      featureFlagsModule.setFeatureFlags(options.featureFlags);

      switch (kind) {
        case 'resolver':
          this.#resolvers.set(specifier, {resolver: instance});
          break;
        case 'transformer': {
          return this.initializeTransformer(instance, specifier);
        }
      }
    },
  );

  runResolverResolve: JsCallable<
    [RunResolverResolveOptions],
    Promise<RunResolverResolveResult>
  > = jsCallable(
    async ({key, dependency: napiDependency, specifier, pipeline}) => {
      const state = this.#resolvers.get(key);
      if (!state) {
        throw new Error(`Resolver not found: ${key}`);
      }

      const env = new Environment(napiDependency.env);
      const dependency = new Dependency(napiDependency, env);

      const defaultOptions = {
        logger: new PluginLogger(),
        tracer: new PluginTracer(),
        options: new PluginOptions(this.options),
      } as const;

      if (!('config' in state)) {
        // @ts-expect-error TS2345
        state.config = await state.resolver.loadConfig?.({
          config: new PluginConfig(
            {
              env: napiDependency.env,
              plugin: key,
              isSource: true,
              searchPath: 'index',
            },
            this.options,
          ),
          ...defaultOptions,
        });
      }

      // @ts-expect-error TS2345
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

      if (result.isExcluded) {
        return {
          invalidations: [],
          resolution: {type: 'excluded'},
        };
      }

      return {
        invalidations: [],
        resolution: {
          type: 'resolved',
          filePath: result.filePath || '',
          canDefer: result.canDefer || false,
          sideEffects: result.sideEffects ?? true,
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
    [RunTransformerTransformOptions, Buffer, string | null | undefined],
    Promise<RunTransformerTransformResult>
  > = jsCallable(async ({key, asset: innerAsset}, contents, map) => {
    const instance = this.#transformers.get(key);
    if (!instance) {
      throw new Error(`Transformer not found: ${key}`);
    }

    let {transformer, config, allowedEnv = new Set()} = instance;

    let cache_bailouts = [];

    const resolveFunc = (from: string, to: string): Promise<any> => {
      let customRequire = module.createRequire(from);
      let resolvedPath = customRequire.resolve(to);
      // Tranformer not cacheable due to use of the resolve function

      cache_bailouts.push(`resolve(${from}, ${to})`);

      return Promise.resolve(resolvedPath);
    };

    const env = new Environment(innerAsset.env);
    let mutableAsset = new MutableAsset(
      innerAsset,
      // @ts-expect-error TS2345
      contents,
      env,
      this.#fs,
      map,
      this.options.projectRoot,
    );

    const pluginOptions = new PluginOptions(this.options);
    const defaultOptions = {
      logger: new PluginLogger(),
      tracer: new PluginTracer(),
      options: pluginOptions,
    } as const;

    if (transformer.loadConfig) {
      if (config != null) {
        throw new Error(
          `Transformer (${key}) should not implement 'setup' and 'loadConfig'`,
        );
      }
      // @ts-expect-error TS2345
      config = await transformer.loadConfig({
        config: new PluginConfig(
          {
            plugin: key,
            isSource: innerAsset.isSource,
            searchPath: innerAsset.filePath,
            env,
          },
          this.options,
        ),
        ...defaultOptions,
      });

      // Transformer uses the deprecated loadConfig API, so mark as not
      // cachable
      cache_bailouts.push(`Transformer.loadConfig`);
    }

    if (transformer.parse) {
      const ast = await transformer.parse({
        // @ts-expect-error TS2322
        asset: mutableAsset,
        config,
        resolve: resolveFunc,
        ...defaultOptions,
      });
      if (ast) {
        mutableAsset.setAST(ast);
      }
      cache_bailouts.push(`Transformer.parse`);
    }

    const [result, sideEffects] =
      await this.#sideEffectDetector.monitorSideEffects(key, () =>
        transformer.transform({
          // @ts-expect-error TS2322
          asset: mutableAsset,
          config,
          resolve: resolveFunc,
          ...defaultOptions,
        }),
      );

    if (sideEffects.envUsage.didEnumerate) {
      cache_bailouts.push(`Env access: enumeration of process.env`);
    }

    for (let variable of sideEffects.envUsage.vars) {
      if (variable in allowedEnv) {
        continue;
      }

      cache_bailouts.push(`Env access: ${variable}`);
    }

    for (let {method, path} of sideEffects.fsUsage) {
      cache_bailouts.push(`FS usage: ${method}(${path})`);
    }

    assert(
      result.length === 1,
      '[V3] Unimplemented: Multiple asset return from Node transformer',
    );

    assert(
      result[0] === mutableAsset,
      '[V3] Unimplemented: New asset returned from Node transformer',
    );

    if (transformer.generate) {
      const ast = await mutableAsset.getAST();
      if (ast) {
        const output = await transformer.generate({
          // @ts-expect-error TS2322
          asset: mutableAsset,
          ast,
          ...defaultOptions,
        });

        if (typeof output.content === 'string') {
          mutableAsset.setCode(output.content);
        } else if (output.content instanceof Buffer) {
          mutableAsset.setBuffer(output.content);
        } else {
          // @ts-expect-error TS2345
          mutableAsset.setStream(output.content);
        }

        if (output.map) {
          mutableAsset.setMap(output.map);
        }
      }
    }

    let assetBuffer: Buffer | null = await mutableAsset.getBuffer();

    // If the asset has no code, we set the buffer to null, which we can
    // detect in Rust, to avoid passing back an empty buffer, which we can't.
    if (assetBuffer.length === 0) {
      assetBuffer = null;
    }

    if (pluginOptions.used) {
      // Plugin options accessed, so not cachable
      cache_bailouts.push(`Plugin options accessed`);
    }

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
      assetBuffer,
      // Only send back the map if it has changed
      mutableAsset.isMapDirty
        ? // @ts-expect-error TS2533
          JSON.stringify((await mutableAsset.getMap()).toVLQ())
        : '',
      // Limit to first 10 bailouts
      // TODO limit has been temporarily removed
      cache_bailouts,
    ];
  });

  get options() {
    if (this.#options == null) {
      throw new Error('Plugin options have not been initialized');
    }
    return this.#options;
  }

  async initializeTransformer(instance: Transformer<any>, specifier: string) {
    let transformer = instance;
    let setup, config, allowedEnv;

    let packageManager = new NodePackageManager(
      this.#fs,
      this.options.projectRoot,
    );

    if (transformer.setup) {
      let setupResult = await transformer.setup({
        logger: new PluginLogger(),
        options: new PluginOptions({
          ...this.options,
          shouldAutoInstall: false,
          inputFS: this.#fs,
          outputFS: this.#fs,
          packageManager,
        }),
        config: new PluginConfig(
          {
            plugin: specifier,
            searchPath: 'index',
            // Consider project setup config as source
            isSource: true,
          },
          this.options,
        ),
      });
      config = setupResult?.config;
      allowedEnv = Object.fromEntries(
        setupResult?.env?.map((env) => [env, process.env[env]]) || [],
      );

      // Always add the following env vars to the cache key
      allowedEnv['NODE_ENV'] = process.env['NODE_ENV'];

      setup = {
        conditions: setupResult?.conditions,
        config,
        env: allowedEnv,
      };
    }

    this.#transformers.set(specifier, {
      transformer,
      config,
      packageManager,
      allowedEnv,
    });

    return setup;
  }
}

// Create napi worker and send it back to main thread
const worker = new AtlaspackWorker();
const napiWorker = napi.newNodejsWorker(worker);
parentPort?.postMessage(napiWorker);

parentPort?.setMaxListeners(parentPort.getMaxListeners() + 1);
parentPort?.addListener('message', (message: unknown) => {
  if (message === 'clearState') {
    worker.clearState();
    parentPort?.postMessage('stateCleared');
  }
});

type ResolverState<T> = {
  resolver: Resolver<T>;
  config?: T;
  packageManager?: NodePackageManager;
};

type TransformerState<ConfigType> = {
  packageManager?: NodePackageManager;
  transformer: Transformer<ConfigType>;
  config?: ConfigType;
  allowedEnv?: Record<string, string | undefined>;
};

type LoadPluginOptions = {
  kind: 'resolver' | 'transformer';
  specifier: string;
  resolveFrom: string;
  options: RpcPluginOptions;
};

type RpcPluginOptions = {
  projectRoot: string;
  mode: string;
  featureFlags: FeatureFlags;
};

type Options = RpcPluginOptions & {
  inputFS: FileSystem;
  outputFS: FileSystem;
  packageManager: NodePackageManager;
  shouldAutoInstall: boolean;
};

type RunResolverResolveOptions = {
  key: string;
  // @ts-expect-error TS2694
  dependency: napi.Dependency;
  specifier: FilePath;
  pipeline: string | null | undefined;
};

type RunResolverResolveResult = {
  invalidations: Array<any>;
  resolution:
    | {
        type: 'unresolved';
      }
    | {
        type: 'excluded';
      }
    | {
        type: 'resolved';
        canDefer: boolean;
        filePath: string;
        sideEffects: boolean;
        code?: string;
        meta?: unknown;
        pipeline?: string;
        priority?: number | null | undefined;
        query?: string;
      };
};

type RunTransformerTransformOptions = {
  key: string;
  // @ts-expect-error TS2724
  env: napi.Environment;
  // @ts-expect-error TS2694
  asset: napi.Asset;
};

type RunTransformerTransformResult = [
  // @ts-expect-error TS2694
  napi.RpcAssetResult,
  Buffer,
  string,
  boolean,
];
