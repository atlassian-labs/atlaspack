import type {
  Config as IPluginConfig,
  DevDepOptions,
  FilePath,
  Environment,
  FileCreateInvalidation,
  ConfigResultWithFilePath,
  PackageJSON,
} from '@atlaspack/types';

import ClassicPublicConfig from '../../../public/Config';
import {type ConfigOpts, createConfig} from '../../../InternalConfig';

export class PluginConfig implements IPluginConfig {
  isSource: boolean;
  searchPath: FilePath;
  env: Environment;
  #inner: ClassicPublicConfig;

  constructor(configOpts: ConfigOpts, options: any) {
    let internalConfig = createConfig(configOpts);

    this.isSource = internalConfig.isSource;
    this.searchPath = internalConfig.searchPath;
    // @ts-expect-error TS2564
    this.env = internalConfig.env;
    this.#inner = new ClassicPublicConfig(internalConfig, options);
  }

  // eslint-disable-next-line no-unused-vars
  invalidateOnFileChange(filePath: FilePath): void {}

  // eslint-disable-next-line no-unused-vars
  invalidateOnFileCreate(invalidations: FileCreateInvalidation): void {}

  // eslint-disable-next-line no-unused-vars
  invalidateOnEnvChange(invalidation: string): void {}

  invalidateOnStartup(): void {}

  invalidateOnBuild(): void {}

  // eslint-disable-next-line no-unused-vars
  addDevDependency(options: DevDepOptions): void {}

  // eslint-disable-next-line no-unused-vars
  setCacheKey(key: string): void {}

  getConfig<T>(
    // eslint-disable-next-line no-unused-vars
    filePaths: Array<FilePath>,
    // eslint-disable-next-line no-unused-vars
    options?: {
      packageKey?: string;
      parse?: boolean;
      exclude?: boolean;
    },
  ): Promise<ConfigResultWithFilePath<T> | null | undefined> {
    return this.#inner.getConfig(filePaths, options);
  }

  getConfigFrom<T>(
    // eslint-disable-next-line no-unused-vars
    searchPath: FilePath,
    // eslint-disable-next-line no-unused-vars
    filePaths: Array<FilePath>,
    // eslint-disable-next-line no-unused-vars
    options?:
      | {
          packageKey?: string;
          parse?: boolean;
          exclude?: boolean;
        }
      | {
          readTracking?: boolean;
        },
  ): Promise<ConfigResultWithFilePath<T> | null | undefined> {
    return this.#inner.getConfigFrom(searchPath, filePaths, options);
  }

  getPackage(): Promise<PackageJSON | null | undefined> {
    return this.#inner.getPackage();
  }
}
