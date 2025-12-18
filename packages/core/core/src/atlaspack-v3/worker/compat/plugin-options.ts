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
  used = false;

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
    this.used = true;
    return this.#options.projectRoot;
  }

  get packageManager(): PackageManager {
    if (!('packageManager' in this.#options)) {
      throw new Error('PluginOptions.packageManager');
    }
    this.used = true;
    return this.#options.packageManager;
  }

  get mode(): BuildMode {
    if (!('mode' in this.#options)) {
      throw new Error('PluginOptions.mode');
    }
    this.used = true;
    return this.#options.mode;
  }

  get parcelVersion(): string {
    if (!('parcelVersion' in this.#options)) {
      return 'UNKNOWN VERSION';
      // throw new Error('PluginOptions.parcelVersion');
    }
    this.used = true;
    return this.#options.parcelVersion;
  }

  get hmrOptions(): HMROptions | null | undefined {
    if (!('hmrOptions' in this.#options)) {
      throw new Error('PluginOptions.hmrOptions');
    }
    this.used = true;
    return this.#options.hmrOptions;
  }

  get serveOptions(): ServerOptions | false {
    if (!('serveOptions' in this.#options)) {
      throw new Error('PluginOptions.serveOptions');
    }
    this.used = true;
    return this.#options.serveOptions;
  }

  get shouldBuildLazily(): boolean {
    if (!('shouldBuildLazily' in this.#options)) {
      throw new Error('PluginOptions.shouldBuildLazily');
    }
    this.used = true;
    return this.#options.shouldBuildLazily;
  }

  get shouldAutoInstall(): boolean {
    if (!('shouldAutoInstall' in this.#options)) {
      throw new Error('PluginOptions.shouldAutoInstall');
    }
    this.used = true;
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
    this.used = true;
    return this.#options.cacheDir;
  }

  get inputFS(): FileSystem {
    if (!('inputFS' in this.#options)) {
      throw new Error('PluginOptions.inputFS');
    }
    this.used = true;
    return this.#options.inputFS;
  }

  get outputFS(): FileSystem {
    if (!('outputFS' in this.#options)) {
      throw new Error('PluginOptions.outputFS');
    }
    this.used = true;
    return this.#options.outputFS;
  }

  get instanceId(): string {
    if (!('instanceId' in this.#options)) {
      throw new Error('PluginOptions.instanceId');
    }
    this.used = true;
    return this.#options.instanceId;
  }

  get detailedReport(): DetailedReportOptions | null | undefined {
    if (!('detailedReport' in this.#options)) {
      throw new Error('PluginOptions.detailedReport');
    }
    this.used = true;
    return this.#options.detailedReport;
  }

  get featureFlags(): FeatureFlags {
    if (!('featureFlags' in this.#options)) {
      throw new Error('PluginOptions.featureFlags');
    }
    this.used = true;
    return this.#options.featureFlags;
  }

  constructor(options: Partial<IPluginOptions>) {
    // @ts-expect-error TS2322
    this.#options = options;
  }
}
