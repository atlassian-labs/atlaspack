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
  // @ts-expect-error TS2564
  #options: AtlaspackOptions;
  #customEnv: EnvMap | null | undefined;

  constructor(
    options: AtlaspackOptions,
    customEnv?: EnvMap | null | undefined,
  ) {
    // Don't use WeakMap caching when custom env is provided, as each target may have different env.
    // The WeakMap uses AtlaspackOptions as the key, but multiple targets can share the same
    // AtlaspackOptions while having different custom environments. Using the cache would cause
    // Target A with {MY_ENV: "one"} to incorrectly reuse a cached PluginOptions from Target B
    // with {MY_ENV: "two"}, leading to wrong environment variables in transformers.
    // Trade-off: Avoids cache for correctness when custom env is used, but has zero overhead
    // when the feature is not used (most common case).
    if (customEnv) {
      this.#options = options;
      this.#customEnv = customEnv;
      return this;
    }

    let existing = parcelOptionsToPluginOptions.get(options);
    if (existing != null) {
      return existing;
    }

    this.#options = options;
    this.#customEnv = null;
    parcelOptionsToPluginOptions.set(options, this);
    return this;
  }

  get instanceId(): string {
    return this.#options.instanceId;
  }

  get mode(): BuildMode {
    return this.#options.mode;
  }

  get env(): EnvMap {
    // Merge global env with custom env from target, with custom env taking precedence
    if (this.#customEnv) {
      return {
        ...this.#options.env,
        ...this.#customEnv,
      };
    }
    return this.#options.env;
  }

  get parcelVersion(): string {
    return this.#options.parcelVersion;
  }

  get hmrOptions(): HMROptions | null | undefined {
    return this.#options.hmrOptions;
  }

  get serveOptions(): ServerOptions | false {
    return this.#options.serveOptions;
  }

  get shouldBuildLazily(): boolean {
    return this.#options.shouldBuildLazily;
  }

  get shouldAutoInstall(): boolean {
    return this.#options.shouldAutoInstall;
  }

  get logLevel(): LogLevel {
    return this.#options.logLevel;
  }

  get cacheDir(): FilePath {
    // TODO: remove this. Probably bad if there are other types of caches.
    // Maybe expose the Cache object instead?
    return this.#options.cacheDir;
  }

  get projectRoot(): FilePath {
    return this.#options.projectRoot;
  }

  get inputFS(): FileSystem {
    return this.#options.inputFS;
  }

  get outputFS(): FileSystem {
    return this.#options.outputFS;
  }

  get packageManager(): PackageManager {
    return this.#options.packageManager;
  }

  get detailedReport(): DetailedReportOptions | null | undefined {
    return this.#options.detailedReport;
  }

  get featureFlags(): FeatureFlags {
    return this.#options.featureFlags;
  }
}
