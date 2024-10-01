import type {
  BuildMode,
  EnvMap,
  FilePath,
  LogLevel,
  PluginOptions as IPluginOptions,
  ServerOptions,
  HMROptions,
  DetailedReportOptions,
} from '@atlaspack/types';
import type {FileSystem} from '@atlaspack/fs';
import type {PackageManager} from '@atlaspack/package-manager';
import type {AtlaspackOptions} from '../types';
import {FeatureFlags} from '@atlaspack/feature-flags';

let parcelOptionsToPluginOptions: WeakMap<AtlaspackOptions, PluginOptions> =
  new WeakMap();

export default class PluginOptions implements IPluginOptions {
  #options /*: AtlaspackOptions */;

  constructor(options: AtlaspackOptions) {
    let existing = parcelOptionsToPluginOptions.get(options);
    if (existing != null) {
      return existing;
    }

    this.#options = options;
    parcelOptionsToPluginOptions.set(options, this);
    return this;
  }

  get instanceId(): string {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#options.instanceId;
  }

  get mode(): BuildMode {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#options.mode;
  }

  get env(): EnvMap {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#options.env;
  }

  get parcelVersion(): string {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#options.parcelVersion;
  }

  get hmrOptions(): HMROptions | null | undefined {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#options.hmrOptions;
  }

  get serveOptions(): ServerOptions | false {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#options.serveOptions;
  }

  get shouldBuildLazily(): boolean {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#options.shouldBuildLazily;
  }

  get shouldAutoInstall(): boolean {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#options.shouldAutoInstall;
  }

  get logLevel(): LogLevel {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#options.logLevel;
  }

  get cacheDir(): FilePath {
    // TODO: remove this. Probably bad if there are other types of caches.
    // Maybe expose the Cache object instead?
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#options.cacheDir;
  }

  get projectRoot(): FilePath {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#options.projectRoot;
  }

  get inputFS(): FileSystem {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#options.inputFS;
  }

  get outputFS(): FileSystem {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#options.outputFS;
  }

  get packageManager(): PackageManager {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#options.packageManager;
  }

  get detailedReport(): DetailedReportOptions | null | undefined {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#options.detailedReport;
  }

  get featureFlags(): FeatureFlags {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#options.featureFlags;
  }
}
