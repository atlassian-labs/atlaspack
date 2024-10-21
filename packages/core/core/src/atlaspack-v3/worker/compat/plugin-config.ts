import type {
  Config as IPluginConfig,
  DevDepOptions,
  FilePath,
  Environment,
  FileCreateInvalidation,
  ConfigResultWithFilePath,
  PackageJSON,
} from '@atlaspack/types';

export type PluginConfigOptions = {
  isSource: boolean,
  searchPath: FilePath,
  env: Environment
};

export class PluginConfig implements IPluginConfig {
  isSource: boolean;
  searchPath: FilePath;
  env: Environment;

  constructor({
    env,
    isSource,
    searchPath,
  }: PluginConfigOptions) {
    this.env = env;
    this.isSource = isSource;
    this.searchPath = searchPath;
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
    options?: {
      packageKey?: string,
      parse?: boolean,
      exclude?: boolean
    },
  ): Promise<ConfigResultWithFilePath<T> | null | undefined> {
    throw new Error('PluginOptions.getConfig');
  }

  getConfigFrom<T>(
    // eslint-disable-next-line no-unused-vars
    searchPath: FilePath,
    // eslint-disable-next-line no-unused-vars
    filePaths: Array<FilePath>,
    // eslint-disable-next-line no-unused-vars
    options?: {
      packageKey?: string,
      parse?: boolean,
      exclude?: boolean
    },
  ): Promise<ConfigResultWithFilePath<T> | null | undefined> {
    throw new Error('PluginOptions.getConfigFrom');
  }

  getPackage(): Promise<PackageJSON | null | undefined> {
    throw new Error('PluginOptions.getPackage');
  }
}
