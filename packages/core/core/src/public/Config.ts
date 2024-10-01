import type {
  Config as IConfig,
  ConfigResult,
  FileCreateInvalidation,
  FilePath,
  PackageJSON,
  ConfigResultWithFilePath,
  DevDepOptions,
} from '@atlaspack/types';
import type {Config, AtlaspackOptions} from '../types';

import invariant from 'assert';
import path from 'path';
import {
  DefaultWeakMap,
  resolveConfig,
  readConfig,
  relativePath,
} from '@atlaspack/utils';
import Environment from './Environment';
import {fromProjectPath, toProjectPath} from '../projectPath';

const internalConfigToConfig: DefaultWeakMap<
  AtlaspackOptions,
  WeakMap<Config, PublicConfig>
> = new DefaultWeakMap(() => new WeakMap());

export default class PublicConfig implements IConfig {
  #config /*: Config */;
  // @ts-expect-error - TS7008 - Member '#pkg' implicitly has an 'any' type.
  #pkg /*: ?PackageJSON */;
  // @ts-expect-error - TS7008 - Member '#pkgFilePath' implicitly has an 'any' type.
  #pkgFilePath /*: ?FilePath */;
  #options /*: AtlaspackOptions */;

  constructor(config: Config, options: AtlaspackOptions) {
    let existing = internalConfigToConfig.get(options).get(config);
    if (existing != null) {
      return existing;
    }

    this.#config = config;
    this.#options = options;
    internalConfigToConfig.get(options).set(config, this);
    return this;
  }

  get env(): Environment {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'. | TS2345 - Argument of type 'AtlaspackOptions | undefined' is not assignable to parameter of type 'AtlaspackOptions'.
    return new Environment(this.#config.env, this.#options);
  }

  get searchPath(): FilePath {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'. | TS2532 - Object is possibly 'undefined'.
    return fromProjectPath(this.#options.projectRoot, this.#config.searchPath);
  }

  get result(): ConfigResult {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#config.result;
  }

  get isSource(): boolean {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#config.isSource;
  }

  // $FlowFixMe
  setResult(result: any): void {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    this.#config.result = result;
  }

  setCacheKey(cacheKey: string) {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    this.#config.cacheKey = cacheKey;
  }

  invalidateOnFileChange(filePath: FilePath) {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    this.#config.invalidateOnFileChange.add(
      // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
      toProjectPath(this.#options.projectRoot, filePath),
    );
  }

  invalidateOnConfigKeyChange(filePath: FilePath, configKey: string) {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    this.#config.invalidateOnConfigKeyChange.push({
      // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
      filePath: toProjectPath(this.#options.projectRoot, filePath),
      configKey,
    });
  }

  addDevDependency(devDep: DevDepOptions) {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    this.#config.devDeps.push({
      ...devDep,
      // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
      resolveFrom: toProjectPath(this.#options.projectRoot, devDep.resolveFrom),
      additionalInvalidations: devDep.additionalInvalidations?.map(
        (i: {
          range?: SemverRange | null | undefined;
          resolveFrom: FilePath;
          specifier: DependencySpecifier;
        }) => ({
          ...i,
          // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
          resolveFrom: toProjectPath(this.#options.projectRoot, i.resolveFrom),
        }),
      ),
    });
  }

  invalidateOnFileCreate(invalidation: FileCreateInvalidation) {
    // @ts-expect-error - TS2339 - Property 'glob' does not exist on type 'FileCreateInvalidation'.
    if (invalidation.glob != null) {
      // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
      this.#config.invalidateOnFileCreate.push(invalidation);
      // @ts-expect-error - TS2339 - Property 'filePath' does not exist on type 'FileCreateInvalidation'.
    } else if (invalidation.filePath != null) {
      // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
      this.#config.invalidateOnFileCreate.push({
        filePath: toProjectPath(
          // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
          this.#options.projectRoot,
          // @ts-expect-error - TS2339 - Property 'filePath' does not exist on type 'FileCreateInvalidation'.
          invalidation.filePath,
        ),
      });
    } else {
      // @ts-expect-error - TS2339 - Property 'aboveFilePath' does not exist on type 'FileCreateInvalidation'.
      invariant(invalidation.aboveFilePath != null);
      // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
      this.#config.invalidateOnFileCreate.push({
        // $FlowFixMe
        // @ts-expect-error - TS2339 - Property 'fileName' does not exist on type 'FileCreateInvalidation'.
        fileName: invalidation.fileName,
        aboveFilePath: toProjectPath(
          // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
          this.#options.projectRoot,
          // @ts-expect-error - TS2339 - Property 'aboveFilePath' does not exist on type 'FileCreateInvalidation'.
          invalidation.aboveFilePath,
        ),
      });
    }
  }

  invalidateOnEnvChange(env: string) {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    this.#config.invalidateOnEnvChange.add(env);
  }

  invalidateOnStartup() {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    this.#config.invalidateOnStartup = true;
  }

  invalidateOnBuild() {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    this.#config.invalidateOnBuild = true;
  }

  async getConfigFrom<T>(
    searchPath: FilePath,
    fileNames: Array<string>,
    options?: {
      packageKey?: string;
      parse?: boolean;
      exclude?: boolean;
    } | null,
  ): Promise<ConfigResultWithFilePath<T> | null | undefined> {
    let packageKey = options?.packageKey;
    if (packageKey != null) {
      let pkg = await this.getConfigFrom(searchPath, ['package.json'], {
        exclude: true,
      });

      // @ts-expect-error - TS2571 - Object is of type 'unknown'.
      if (pkg && pkg.contents[packageKey]) {
        // Invalidate only when the package key changes
        this.invalidateOnConfigKeyChange(pkg.filePath, packageKey);

        return {
          // @ts-expect-error - TS2571 - Object is of type 'unknown'.
          contents: pkg.contents[packageKey],
          filePath: pkg.filePath,
        };
      }
    }

    if (fileNames.length === 0) {
      return null;
    }

    // Invalidate when any of the file names are created above the search path.
    for (let fileName of fileNames) {
      this.invalidateOnFileCreate({
        fileName,
        aboveFilePath: searchPath,
      });
    }

    let parse = options && options.parse;
    let configFilePath = await resolveConfig(
      // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
      this.#options.inputFS,
      searchPath,
      fileNames,
      // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
      this.#options.projectRoot,
    );
    if (configFilePath == null) {
      return null;
    }

    if (!options || !options.exclude) {
      this.invalidateOnFileChange(configFilePath);
    }

    // If this is a JavaScript file, load it with the package manager.
    let extname = path.extname(configFilePath);
    if (extname === '.js' || extname === '.cjs' || extname === '.mjs') {
      let specifier = relativePath(path.dirname(searchPath), configFilePath);

      // Add dev dependency so we reload the config and any dependencies in watch mode.
      this.addDevDependency({
        specifier,
        resolveFrom: searchPath,
      });

      // Invalidate on startup in case the config is non-deterministic,
      // e.g. uses unknown environment variables, reads from the filesystem, etc.
      this.invalidateOnStartup();

      // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
      let config = await this.#options.packageManager.require(
        specifier,
        searchPath,
      );

      if (
        // $FlowFixMe
        Object.prototype.toString.call(config) === '[object Module]' &&
        config.default != null
      ) {
        // Native ESM config. Try to use a default export, otherwise fall back to the whole namespace.
        config = config.default;
      }

      return {
        contents: config,
        filePath: configFilePath,
      };
    }

    let conf = await readConfig(
      // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
      this.#options.inputFS,
      configFilePath,
      parse == null ? null : {parse},
    );
    if (conf == null) {
      return null;
    }

    return {
      contents: conf.config,
      filePath: configFilePath,
    };
  }

  getConfig<T>(
    filePaths: Array<FilePath>,
    options?: {
      packageKey?: string;
      parse?: boolean;
      exclude?: boolean;
    } | null,
  ): Promise<ConfigResultWithFilePath<T> | null | undefined> {
    return this.getConfigFrom(this.searchPath, filePaths, options);
  }

  async getPackage(): Promise<PackageJSON | null | undefined> {
    if (this.#pkg) {
      return this.#pkg;
    }

    let pkgConfig = await this.getConfig<PackageJSON>(['package.json']);
    if (!pkgConfig) {
      return null;
    }

    this.#pkg = pkgConfig.contents;
    this.#pkgFilePath = pkgConfig.filePath;

    return this.#pkg;
  }
}
