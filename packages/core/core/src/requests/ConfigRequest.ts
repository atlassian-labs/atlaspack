import type {
  Async,
  Config as IConfig,
  PluginOptions as IPluginOptions,
  PluginLogger as IPluginLogger,
  PluginTracer as IPluginTracer,
  NamedBundle as INamedBundle,
  BundleGraph as IBundleGraph,
} from '@atlaspack/types';
import {readConfig, hashObject} from '@atlaspack/utils';
import type {
  Config,
  AtlaspackOptions,
  InternalFileCreateInvalidation,
} from '../types';
import type {LoadedPlugin} from '../AtlaspackConfig';
import type {RequestResult, RunAPI} from '../RequestTracker';
import type {ProjectPath} from '../projectPath';

import {createBuildCache, serializeRaw} from '@atlaspack/build-cache';
import {PluginLogger} from '@atlaspack/logger';
import PluginOptions from '../public/PluginOptions';
import ThrowableDiagnostic, {errorToDiagnostic} from '@atlaspack/diagnostic';
import PublicConfig from '../public/Config';
import {optionsProxy} from '../utils';
import {getInvalidationHash} from '../assetUtils';
import {hashString, Hash} from '@atlaspack/rust';
import {PluginTracer} from '@atlaspack/profiler';
import {requestTypes} from '../RequestTracker';
import {fromProjectPath, fromProjectPathRelative} from '../projectPath';

export type PluginWithLoadConfig = {
  loadConfig?: (arg1: {
    config: IConfig;
    options: IPluginOptions;
    logger: IPluginLogger;
    tracer: IPluginTracer;
  }) => Async<unknown>;
};

export type PluginWithBundleConfig = {
  loadConfig?: (arg1: {
    config: IConfig;
    options: IPluginOptions;
    logger: IPluginLogger;
    tracer: IPluginTracer;
  }) => Async<unknown>;
  loadBundleConfig?: (arg1: {
    bundle: INamedBundle;
    bundleGraph: IBundleGraph<INamedBundle>;
    config: IConfig;
    options: IPluginOptions;
    logger: IPluginLogger;
    tracer: IPluginTracer;
  }) => Async<unknown>;
};

export type ConfigRequest = {
  id: string;
  invalidateOnFileChange: Set<ProjectPath>;
  invalidateOnConfigKeyChange: Array<{
    filePath: ProjectPath;
    configKey: string[];
  }>;
  invalidateOnFileCreate: Array<InternalFileCreateInvalidation>;
  invalidateOnEnvChange: Set<string>;
  invalidateOnOptionChange: Set<string>;
  invalidateOnStartup: boolean;
  invalidateOnBuild: boolean;
};

export type ConfigRequestResult = undefined;

export async function loadPluginConfig<T extends PluginWithLoadConfig>(
  loadedPlugin: LoadedPlugin<T>,
  config: Config,
  options: AtlaspackOptions,
): Promise<void> {
  let loadConfig = loadedPlugin.plugin.loadConfig;
  if (!loadConfig) {
    return;
  }

  try {
    config.result = await loadConfig({
      config: new PublicConfig(config, options),
      options: new PluginOptions(
        optionsProxy(options, (option) => {
          config.invalidateOnOptionChange.add(option);
        }),
      ),
      logger: new PluginLogger({origin: loadedPlugin.name}),
      tracer: new PluginTracer({
        origin: loadedPlugin.name,
        category: 'loadConfig',
      }),
    });
  } catch (e: any) {
    throw new ThrowableDiagnostic({
      diagnostic: errorToDiagnostic(e, {
        origin: loadedPlugin.name,
      }),
    });
  }
}

/**
 * Return value at a given key path within an object.
 *
 * @example
 *     const obj = { a: { b: { c: 'd' } } };
 *     getValueAtPath(obj, ['a', 'b', 'c']);        // 'd'
 *     getValueAtPath(obj, ['a', 'b', 'd']);        // undefined
 *     getValueAtPath(obj, ['a', 'b']);             // { c: 'd' }
 *     getValueAtPath(obj, ['a', 'b', 'c', 'd']);   // undefined
 */
export function getValueAtPath(obj: any, key: string[]): any {
  let current = obj;
  for (let part of key) {
    if (current == null) {
      return undefined;
    }
    current = current[part];
  }
  return current;
}

const configKeyCache = createBuildCache();

export async function getConfigKeyContentHash(
  filePath: ProjectPath,
  configKey: string[],
  options: AtlaspackOptions,
): Promise<Async<string>> {
  let cacheKey = `${fromProjectPathRelative(filePath)}:${JSON.stringify(
    configKey,
  )}`;
  let cachedValue = configKeyCache.get(cacheKey);

  if (cachedValue) {
    // @ts-expect-error TS2322
    return cachedValue;
  }

  const conf = await readConfig(
    options.inputFS,
    fromProjectPath(options.projectRoot, filePath),
  );

  const value = getValueAtPath(conf?.config, configKey);
  if (conf == null || value == null) {
    // This can occur when a config key has been removed entirely during `respondToFSEvents`
    return '';
  }

  const contentHash =
    typeof value === 'object'
      ? hashObject(value)
      : hashString(JSON.stringify(value));

  configKeyCache.set(cacheKey, contentHash);

  return contentHash;
}

export async function runConfigRequest<TResult extends RequestResult>(
  api: RunAPI<TResult>,
  configRequest: ConfigRequest,
) {
  let {
    invalidateOnFileChange,
    invalidateOnConfigKeyChange,
    invalidateOnFileCreate,
    invalidateOnEnvChange,
    invalidateOnOptionChange,
    invalidateOnStartup,
    invalidateOnBuild,
  } = configRequest;

  // If there are no invalidations, then no need to create a node.
  if (
    invalidateOnFileChange.size === 0 &&
    invalidateOnConfigKeyChange.length === 0 &&
    invalidateOnFileCreate.length === 0 &&
    invalidateOnOptionChange.size === 0 &&
    invalidateOnEnvChange.size === 0 &&
    !invalidateOnStartup &&
    !invalidateOnBuild
  ) {
    return;
  }

  await api.runRequest<null, undefined>({
    id: 'config_request:' + configRequest.id,
    type: requestTypes.config_request,
    run: async ({api, options}) => {
      for (let filePath of invalidateOnFileChange) {
        api.invalidateOnFileUpdate(filePath);
        api.invalidateOnFileDelete(filePath);
      }

      for (let {filePath, configKey} of invalidateOnConfigKeyChange) {
        let contentHash = await getConfigKeyContentHash(
          filePath,
          configKey,
          options,
        );

        api.invalidateOnConfigKeyChange(filePath, configKey, contentHash);
      }

      for (let invalidation of invalidateOnFileCreate) {
        api.invalidateOnFileCreate(invalidation);
      }

      for (let env of invalidateOnEnvChange) {
        api.invalidateOnEnvChange(env);
      }

      for (let option of invalidateOnOptionChange) {
        api.invalidateOnOptionChange(option);
      }

      if (invalidateOnStartup) {
        api.invalidateOnStartup();
      }

      if (invalidateOnBuild) {
        api.invalidateOnBuild();
      }
    },
    input: null,
  });
}

export async function getConfigHash(
  config: Config,
  pluginName: string,
  options: AtlaspackOptions,
): Promise<string> {
  if (config.result == null) {
    return '';
  }

  let hash = new Hash();
  hash.writeString(config.id);

  // If there is no result hash set by the transformer, default to hashing the included
  // files if any, otherwise try to hash the config result itself.
  if (config.cacheKey == null) {
    if (config.invalidateOnFileChange.size > 0) {
      hash.writeString(
        await getInvalidationHash(
          [...config.invalidateOnFileChange].map((filePath) => ({
            type: 'file',
            filePath,
          })),
          options,
        ),
      );
    } else if (config.result != null) {
      try {
        hash.writeBuffer(serializeRaw(config.result));
      } catch (err: any) {
        throw new ThrowableDiagnostic({
          diagnostic: {
            message:
              'Config result is not hashable because it contains non-serializable objects. Please use config.setCacheKey to set the hash manually.',
            origin: pluginName,
          },
        });
      }
    }
  } else {
    hash.writeString(config.cacheKey ?? '');
  }

  return hash.finish();
}

export function getConfigRequests(
  configs: Array<Config>,
): Array<ConfigRequest> {
  return configs
    .filter((config) => {
      // No need to send to the graph if there are no invalidations.
      return (
        config.invalidateOnFileChange.size > 0 ||
        config.invalidateOnConfigKeyChange.length > 0 ||
        config.invalidateOnFileCreate.length > 0 ||
        config.invalidateOnEnvChange.size > 0 ||
        config.invalidateOnOptionChange.size > 0 ||
        config.invalidateOnStartup ||
        config.invalidateOnBuild
      );
    })
    .map((config) => ({
      id: config.id,
      invalidateOnFileChange: config.invalidateOnFileChange,
      invalidateOnConfigKeyChange: config.invalidateOnConfigKeyChange,
      invalidateOnFileCreate: config.invalidateOnFileCreate,
      invalidateOnEnvChange: config.invalidateOnEnvChange,
      invalidateOnOptionChange: config.invalidateOnOptionChange,
      invalidateOnStartup: config.invalidateOnStartup,
      invalidateOnBuild: config.invalidateOnBuild,
    }));
}
