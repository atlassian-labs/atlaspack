// @flow

import type {
  Config as IPluginConfig,
  DevDepOptions,
  FilePath,
  Environment,
  FileCreateInvalidation,
  ConfigResultWithFilePath,
  PackageJSON,
  PackageManager as IPackageManager,
} from '@atlaspack/types';

import type {FileSystem as IFileSystem} from '@atlaspack/fs';
import ClassicPublicConfig from '../../../public/Config';

export type PluginConfigOptions = {|
  isSource: boolean,
  searchPath: FilePath,
  projectRoot: FilePath,
  env: Environment,
  fs: IFileSystem,
  packageManager: IPackageManager,
|};

export class PluginConfig implements IPluginConfig {
  isSource: boolean;
  searchPath: FilePath;
  #projectRoot: FilePath;
  env: Environment;
  #inner: ClassicPublicConfig;

  constructor({
    env,
    isSource,
    searchPath,
    projectRoot,
    fs,
    packageManager,
  }: PluginConfigOptions) {
    this.env = env;
    this.isSource = isSource;
    this.searchPath = searchPath;

    this.#inner = new ClassicPublicConfig(
      // $FlowFixMe
      {
        invalidateOnConfigKeyChange: [],
        invalidateOnFileCreate: [],
        invalidateOnFileChange: new Set(),
        devDeps: [],
        // $FlowFixMe
        searchPath: searchPath.replace(projectRoot + '/', ''),
      },
      // $FlowFixMe
      {
        projectRoot,
        inputFS: fs,
        outputFS: fs,
        packageManager,
      },
    );
  }

  // eslint-disable-next-line no-unused-vars
  invalidateOnFileChange(filePath: FilePath): void {
    // throw new Error('PluginOptions.invalidateOnFileChange');
  }

  // eslint-disable-next-line no-unused-vars
  invalidateOnFileCreate(invalidations: FileCreateInvalidation): void {
    // throw new Error('PluginOptions.invalidateOnFileCreate');
  }

  // eslint-disable-next-line no-unused-vars
  invalidateOnEnvChange(invalidation: string): void {
    // throw new Error('PluginOptions.invalidateOnEnvChange');
  }

  invalidateOnStartup(): void {
    throw new Error('PluginOptions.invalidateOnStartup');
  }

  invalidateOnBuild(): void {
    // throw new Error('PluginOptions.invalidateOnBuild');
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
    return this.#inner.getConfig(filePaths, options);
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
    return this.#inner.getConfigFrom(searchPath, filePaths, options);
  }

  getPackage(): Promise<?PackageJSON> {
    return this.#inner.getPackage();
  }
}
