// @flow

import type {
  Config as IPluginConfig,
  DevDepOptions,
  FilePath,
  Environment,
  FileCreateInvalidation,
  ConfigResultWithFilePath,
  PackageJSON,
} from '@atlaspack/types';

export type PluginConfigOptions = {|
  isSource: boolean,
  searchPath: FilePath,
  env: Environment,
|};

interface ConfigLoader {
  loadJsonConfig(filePath: string): any;
  loadJsonConfigFrom(searchPath: string, filePath: string): any;
}

export class PluginConfig implements IPluginConfig {
  isSource: boolean;
  searchPath: FilePath;
  env: Environment;
  #configLoader: ConfigLoader;

  constructor(
    configLoader: ConfigLoader,
    {env, isSource, searchPath}: PluginConfigOptions,
  ) {
    this.env = env;
    this.isSource = isSource;
    this.searchPath = searchPath;
    this.#configLoader = configLoader;
  }

  // eslint-disable-next-line no-unused-vars
  invalidateOnFileChange(filePath: FilePath): void {
    throw new Error('PluginOptions.invalidateOnFileChange');
  }

  // eslint-disable-next-line no-unused-vars
  invalidateOnFileCreate(invalidations: FileCreateInvalidation): void {
    throw new Error('PluginOptions.invalidateOnFileCreate');
  }

  // eslint-disable-next-line no-unused-vars
  invalidateOnEnvChange(invalidation: string): void {
    throw new Error('PluginOptions.invalidateOnEnvChange');
  }

  invalidateOnStartup(): void {
    throw new Error('PluginOptions.invalidateOnStartup');
  }

  invalidateOnBuild(): void {
    throw new Error('PluginOptions.invalidateOnBuild');
  }

  // eslint-disable-next-line no-unused-vars
  addDevDependency(options: DevDepOptions): void {
    throw new Error('PluginOptions.addDevDependency');
  }

  // eslint-disable-next-line no-unused-vars
  setCacheKey(key: string): void {
    throw new Error('PluginOptions.setCacheKey');
  }

  getConfig<T>(
    // eslint-disable-next-line no-unused-vars
    filePaths: Array<FilePath>,
    // eslint-disable-next-line no-unused-vars
    options?: {|
      packageKey?: string,
      parse?: boolean,
      exclude?: boolean,
    |},
  ): Promise<?ConfigResultWithFilePath<T>> {
    for (const filePath of filePaths) {
      const found = this.#configLoader.loadJsonConfig(filePath);
      if (found) {
        return Promise.resolve({
          contents: found.contents,
          filePath: found.path,
        });
      }
    }
    throw new Error('PluginOptions.getConfig');
  }

  getConfigFrom<T>(
    // eslint-disable-next-line no-unused-vars
    searchPath: FilePath,
    // eslint-disable-next-line no-unused-vars
    filePaths: Array<FilePath>,
    // eslint-disable-next-line no-unused-vars
    options?: {|
      packageKey?: string,
      parse?: boolean,
      exclude?: boolean,
    |},
  ): Promise<?ConfigResultWithFilePath<T>> {
    for (const filePath of filePaths) {
      const found = this.#configLoader.loadJsonConfigFrom(searchPath, filePath);
      if (found) {
        return Promise.resolve({
          contents: found.contents,
          filePath: found.path,
        });
      }
    }

    throw new Error('PluginOptions.getConfigFrom');
  }

  getPackage(): Promise<?PackageJSON> {
    throw new Error('PluginOptions.getPackage');
  }
}
