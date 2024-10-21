import type {
  PluginOptions as IPluginOptions,
  LogLevel,
  FileSystem,
  PackageManager,
  EnvMap,
  FilePath,
  HMROptions,
  ServerOptions,
  DetailedReportOptions,
  BuildMode,
} from '@atlaspack/types';
import type {FeatureFlags} from '@atlaspack/feature-flags';

export class PluginOptions implements IPluginOptions {
  #options: IPluginOptions;

  get env(): EnvMap {
    if (!('env' in this.#options)) {
      return process.env;
      // throw new Error('PluginOptions.env');
    }
    return this.#options.env;
  }

  get projectRoot(): FilePath {
    if (!('projectRoot' in this.#options)) {
      throw new Error('PluginOptions.projectRoot');
    }
    return this.#options.projectRoot;
  }

  get packageManager(): PackageManager {
    if (!('packageManager' in this.#options)) {
      throw new Error('PluginOptions.packageManager');
    }
    return this.#options.packageManager;
  }

  get mode(): BuildMode {
    if (!('mode' in this.#options)) {
      throw new Error('PluginOptions.mode');
    }
    return this.#options.mode;
  }

  get parcelVersion(): string {
    if (!('parcelVersion' in this.#options)) {
      return 'UNKNOWN VERSION';
      // throw new Error('PluginOptions.parcelVersion');
    }
    return this.#options.parcelVersion;
  }

  get hmrOptions(): HMROptions | null | undefined {
    if (!('hmrOptions' in this.#options)) {
      throw new Error('PluginOptions.hmrOptions');
    }
    return this.#options.hmrOptions;
  }

  get serveOptions(): ServerOptions | false {
    if (!('serveOptions' in this.#options)) {
      throw new Error('PluginOptions.serveOptions');
    }
    return this.#options.serveOptions;
  }

  get shouldBuildLazily(): boolean {
    if (!('shouldBuildLazily' in this.#options)) {
      throw new Error('PluginOptions.shouldBuildLazily');
    }
    return this.#options.shouldBuildLazily;
  }

  get shouldAutoInstall(): boolean {
    if (!('shouldAutoInstall' in this.#options)) {
      throw new Error('PluginOptions.shouldAutoInstall');
    }
    return this.#options.shouldAutoInstall;
  }

  get logLevel(): LogLevel {
    if (!('logLevel' in this.#options)) {
      throw new Error('PluginOptions.logLevel');
    }
    return this.#options.logLevel;
  }

  get cacheDir(): string {
    if (!('cacheDir' in this.#options)) {
      throw new Error('PluginOptions.cacheDir');
    }
    return this.#options.cacheDir;
  }

  get inputFS(): FileSystem {
    if (!('inputFS' in this.#options)) {
      throw new Error('PluginOptions.inputFS');
    }
    return this.#options.inputFS;
  }

  get outputFS(): FileSystem {
    if (!('outputFS' in this.#options)) {
      throw new Error('PluginOptions.outputFS');
    }
    return this.#options.outputFS;
  }

  get instanceId(): string {
    if (!('instanceId' in this.#options)) {
      throw new Error('PluginOptions.instanceId');
    }
    return this.#options.instanceId;
  }

  get detailedReport(): DetailedReportOptions | null | undefined {
    if (!('detailedReport' in this.#options)) {
      throw new Error('PluginOptions.detailedReport');
    }
    return this.#options.detailedReport;
  }

  get featureFlags(): FeatureFlags {
    if (!('featureFlags' in this.#options)) {
      throw new Error('PluginOptions.featureFlags');
    }
    return this.#options.featureFlags;
  }

  constructor(options: Partial<IPluginOptions>) {
    this.#options = options;
  }
}
