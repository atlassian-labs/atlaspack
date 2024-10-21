import type {
  Glob,
  Transformer,
  Resolver,
  Bundler,
  Namer,
  Runtime,
  PackageName,
  Optimizer,
  Compressor,
  Packager,
  Reporter,
  Semver,
  SemverRange,
  Validator,
  FilePath,
} from '@atlaspack/types';
import type {
  ProcessedAtlaspackConfig,
  AtlaspackPluginNode,
  PureAtlaspackConfigPipeline,
  ExtendableAtlaspackConfigPipeline,
  AtlaspackOptions,
} from './types';
import ThrowableDiagnostic, {
  md,
  generateJSONCodeHighlights,
} from '@atlaspack/diagnostic';
import json5 from 'json5';

import {globToRegex} from '@atlaspack/utils';
import {basename} from 'path';
import loadPlugin from './loadAtlaspackPlugin';
import {
  ProjectPath,
  fromProjectPath,
  fromProjectPathRelative,
  toProjectPathUnsafe,
} from './projectPath';

type GlobMap<T> = Partial<Record<Glob, T>>;
type SerializedAtlaspackConfig = {
  $$raw: boolean,
  config: ProcessedAtlaspackConfig,
  options: AtlaspackOptions
};

export type LoadedPlugin<T> = {
  name: string,
  version: Semver,
  plugin: T,
  resolveFrom: ProjectPath,
  keyPath?: string,
  range?: SemverRange | null | undefined
};

export class AtlaspackConfig {
  options: AtlaspackOptions;
  filePath: ProjectPath;
  resolvers: PureAtlaspackConfigPipeline;
  transformers: GlobMap<ExtendableAtlaspackConfigPipeline>;
  bundler: AtlaspackPluginNode | null | undefined;
  namers: PureAtlaspackConfigPipeline;
  runtimes: PureAtlaspackConfigPipeline;
  packagers: GlobMap<AtlaspackPluginNode>;
  validators: GlobMap<ExtendableAtlaspackConfigPipeline>;
  optimizers: GlobMap<ExtendableAtlaspackConfigPipeline>;
  compressors: GlobMap<ExtendableAtlaspackConfigPipeline>;
  reporters: PureAtlaspackConfigPipeline;
  pluginCache: Map<PackageName, any>;
  regexCache: Map<string, RegExp>;

  constructor(config: ProcessedAtlaspackConfig, options: AtlaspackOptions) {
    this.options = options;
    this.filePath = config.filePath;
    this.resolvers = config.resolvers || [];
    this.transformers = config.transformers || {};
    this.runtimes = config.runtimes || [];
    this.bundler = config.bundler;
    this.namers = config.namers || [];
    this.packagers = config.packagers || {};
    this.optimizers = config.optimizers || {};
    this.compressors = config.compressors || {};
    this.reporters = config.reporters || [];
    this.validators = config.validators || {};
    this.pluginCache = new Map();
    this.regexCache = new Map();
  }

  static deserialize(serialized: SerializedAtlaspackConfig): AtlaspackConfig {
    return new AtlaspackConfig(serialized.config, serialized.options);
  }

  getConfig(): ProcessedAtlaspackConfig {
    return {
      filePath: this.filePath,
      resolvers: this.resolvers,
      transformers: this.transformers,
      validators: this.validators,
      runtimes: this.runtimes,
      bundler: this.bundler,
      namers: this.namers,
      packagers: this.packagers,
      optimizers: this.optimizers,
      compressors: this.compressors,
      reporters: this.reporters,
    };
  }

  serialize(): SerializedAtlaspackConfig {
    return {
      $$raw: false,
      config: this.getConfig(),
      options: this.options,
    };
  }

  _loadPlugin<T>(node: AtlaspackPluginNode): Promise<{
    plugin: T,
    version: Semver,
    resolveFrom: ProjectPath,
    range: SemverRange | null | undefined
  }> {
    let plugin = this.pluginCache.get(node.packageName);
    if (plugin) {
      return plugin;
    }

    plugin = loadPlugin<T>(
      node.packageName,
      fromProjectPath(this.options.projectRoot, node.resolveFrom),
      node.keyPath,
      this.options,
    );

    this.pluginCache.set(node.packageName, plugin);
    return plugin;
  }

  async loadPlugin<T>(node: AtlaspackPluginNode): Promise<LoadedPlugin<T>> {
    let plugin = await this._loadPlugin(node);
    return {
      ...plugin,
      name: node.packageName,
      keyPath: node.keyPath,
    };
  }

  invalidatePlugin(packageName: PackageName) {
    this.pluginCache.delete(packageName);
  }

  loadPlugins<T>(plugins: PureAtlaspackConfigPipeline): Promise<Array<LoadedPlugin<T>>> {
    return Promise.all(plugins.map((p) => this.loadPlugin<T>(p)));
  }

  async getResolvers(): Promise<Array<LoadedPlugin<Resolver<unknown>>>> {
    if (this.resolvers.length === 0) {
      throw await this.missingPluginError(
        this.resolvers,
        'No resolver plugins specified in .parcelrc config',
        '/resolvers',
      );
    }

    return this.loadPlugins<Resolver<unknown>>(this.resolvers);
  }

  _getValidatorNodes(filePath: ProjectPath): ReadonlyArray<AtlaspackPluginNode> {
    let validators: PureAtlaspackConfigPipeline =
      this.matchGlobMapPipelines(filePath, this.validators) || [];

    return validators;
  }

  getValidatorNames(filePath: ProjectPath): Array<string> {
    let validators: PureAtlaspackConfigPipeline =
      this._getValidatorNodes(filePath);
    return validators.map((v) => v.packageName);
  }

  getValidators(filePath: ProjectPath): Promise<Array<LoadedPlugin<Validator>>> {
    let validators = this._getValidatorNodes(filePath);
    return this.loadPlugins<Validator>(validators);
  }

  getNamedPipelines(): ReadonlyArray<string> {
    return Object.keys(this.transformers)
      .filter((glob) => glob.includes(':'))
      .map((glob) => glob.split(':')[0]);
  }

  async getTransformers(filePath: ProjectPath, pipeline?: string | null, allowEmpty?: boolean): Promise<Array<LoadedPlugin<Transformer<unknown>>>> {
    let transformers: PureAtlaspackConfigPipeline | null =
      this.matchGlobMapPipelines(filePath, this.transformers, pipeline);
    if (!transformers || transformers.length === 0) {
      if (allowEmpty) {
        return [];
      }

      throw await this.missingPluginError(
        this.transformers,
        md`No transformers found for __${fromProjectPathRelative(filePath)}__` +
          (pipeline != null ? ` with pipeline: '${pipeline}'` : '') +
          '.',
        '/transformers',
      );
    }

    return this.loadPlugins<Transformer<unknown>>(transformers);
  }

  async getBundler(): Promise<LoadedPlugin<Bundler<unknown>>> {
    if (!this.bundler) {
      throw await this.missingPluginError(
        [],
        'No bundler specified in .parcelrc config',
        '/bundler',
      );
    }

    return this.loadPlugin<Bundler<unknown>>(this.bundler);
  }

  async getNamers(): Promise<Array<LoadedPlugin<Namer<unknown>>>> {
    if (this.namers.length === 0) {
      throw await this.missingPluginError(
        this.namers,
        'No namer plugins specified in .parcelrc config',
        '/namers',
      );
    }

    return this.loadPlugins<Namer<unknown>>(this.namers);
  }

  getRuntimes(): Promise<Array<LoadedPlugin<Runtime<unknown>>>> {
    if (!this.runtimes) {
      return Promise.resolve([]);
    }

    return this.loadPlugins<Runtime<unknown>>(this.runtimes);
  }

  async getPackager(filePath: FilePath): Promise<LoadedPlugin<Packager<unknown, unknown>>> {
    let packager = this.matchGlobMap(
      toProjectPathUnsafe(filePath),
      this.packagers,
    );
    if (!packager) {
      throw await this.missingPluginError(
        this.packagers,
        md`No packager found for __${filePath}__.`,
        '/packagers',
      );
    }
    return this.loadPlugin<Packager<unknown, unknown>>(packager);
  }

  _getOptimizerNodes(filePath: FilePath, pipeline?: string | null): PureAtlaspackConfigPipeline {
    // If a pipeline is specified, but it doesn't exist in the optimizers config, ignore it.
    // Pipelines for bundles come from their entry assets, so the pipeline likely exists in transformers.
    if (pipeline) {
      let prefix = pipeline + ':';
      if (
        !Object.keys(this.optimizers).some((glob) => glob.startsWith(prefix))
      ) {
        pipeline = null;
      }
    }

    return (
      this.matchGlobMapPipelines(
        toProjectPathUnsafe(filePath),
        this.optimizers,
        pipeline,
      ) ?? []
    );
  }

  getOptimizerNames(filePath: FilePath, pipeline?: string | null): Array<string> {
    let optimizers = this._getOptimizerNodes(filePath, pipeline);
    return optimizers.map((o) => o.packageName);
  }

  getOptimizers(filePath: FilePath, pipeline?: string | null): Promise<Array<LoadedPlugin<Optimizer<unknown, unknown>>>> {
    let optimizers = this._getOptimizerNodes(filePath, pipeline);
    if (optimizers.length === 0) {
      return Promise.resolve([]);
    }

    return this.loadPlugins<Optimizer<unknown, unknown>>(optimizers);
  }

  async getCompressors(filePath: FilePath): Promise<Array<LoadedPlugin<Compressor>>> {
    let compressors =
      this.matchGlobMapPipelines(
        toProjectPathUnsafe(filePath),
        this.compressors,
      ) ?? [];

    if (compressors.length === 0) {
      throw await this.missingPluginError(
        this.compressors,
        md`No compressors found for __${filePath}__.`,
        '/compressors',
      );
    }

    return this.loadPlugins<Compressor>(compressors);
  }

  getReporters(): Promise<Array<LoadedPlugin<Reporter>>> {
    return this.loadPlugins<Reporter>(this.reporters);
  }

  isGlobMatch(projectPath: ProjectPath, pattern: Glob, pipeline?: string | null): boolean {
    // glob's shouldn't be dependant on absolute paths anyway
    let filePath = fromProjectPathRelative(projectPath);

    let [patternPipeline, patternGlob] = pattern.split(':');
    if (!patternGlob) {
      patternGlob = patternPipeline;
      patternPipeline = null;
    }

    let re = this.regexCache.get(patternGlob);
    if (!re) {
      re = globToRegex(patternGlob, {dot: true, nocase: true});
      this.regexCache.set(patternGlob, re);
    }

    return (
      (pipeline === patternPipeline || (!pipeline && !patternPipeline)) &&
      (re.test(filePath) || re.test(basename(filePath)))
    );
  }

  matchGlobMap<T>(filePath: ProjectPath, globMap: Partial<Record<Glob, T>>): T | null | undefined {
    for (let pattern in globMap) {
      if (this.isGlobMatch(filePath, pattern)) {
        return globMap[pattern];
      }
    }

    return null;
  }

  matchGlobMapPipelines(
    filePath: ProjectPath,
    globMap: Partial<Record<Glob, ExtendableAtlaspackConfigPipeline>>,
    pipeline?: string | null,
  ): PureAtlaspackConfigPipeline {
    let matches: Array<ExtendableAtlaspackConfigPipeline> = [];
    if (pipeline) {
      // If a pipeline is requested, a the glob needs to match exactly
      let exactMatch;
      for (let pattern in globMap) {
        if (this.isGlobMatch(filePath, pattern, pipeline)) {
          exactMatch = globMap[pattern];
          break;
        }
      }
      if (!exactMatch) {
        return [];
      } else {
        matches.push(exactMatch);
      }
    }

    for (let pattern in globMap) {
      if (this.isGlobMatch(filePath, pattern)) {
        matches.push(globMap[pattern]);
      }
    }

    let flatten = () => {
      let pipeline = matches.shift() || [];
      let spreadIndex = pipeline.indexOf('...');
      if (spreadIndex >= 0) {
        pipeline = [
          ...pipeline.slice(0, spreadIndex),
          ...flatten(),
          ...pipeline.slice(spreadIndex + 1),
        ];
      }

      if (pipeline.includes('...')) {
        throw new Error(
          'Only one spread parameter can be included in a config pipeline',
        );
      }

      return pipeline;
    };

    let res = flatten();
    return res;
  }

  async missingPluginError(
    plugins: GlobMap<ExtendableAtlaspackConfigPipeline> | GlobMap<AtlaspackPluginNode> | PureAtlaspackConfigPipeline,
    message: string,
    key: string,
  ): Promise<ThrowableDiagnostic> {
    let configsWithPlugin;
    if (Array.isArray(plugins)) {
      configsWithPlugin = new Set(getConfigPaths(this.options, plugins));
    } else {
      configsWithPlugin = new Set(
        Object.keys(plugins).flatMap((k) =>
          Array.isArray(plugins[k])
            ? getConfigPaths(this.options, plugins[k])
            : [getConfigPath(this.options, plugins[k])],
        ),
      );
    }

    if (configsWithPlugin.size === 0) {
      configsWithPlugin.add(
        fromProjectPath(this.options.projectRoot, this.filePath),
      );
    }

    let seenKey = false;
    let codeFrames = await Promise.all(
      [...configsWithPlugin].map(async (filePath) => {
        let configContents = await this.options.inputFS.readFile(
          filePath,
          'utf8',
        );
        if (!json5.parse(configContents)[key.slice(1)]) {
          key = '';
        } else {
          seenKey = true;
        }
        return {
          filePath,
          code: configContents,
          codeHighlights: generateJSONCodeHighlights(configContents, [{key}]),
        };
      }),
    );
    return new ThrowableDiagnostic({
      diagnostic: {
        message,
        origin: '@atlaspack/core',
        codeFrames,
        hints: !seenKey ? ['Try extending __@atlaspack/config-default__'] : [],
      },
    });
  }
}

function getConfigPaths(options: AtlaspackOptions, nodes: AtlaspackPluginNode | PureAtlaspackConfigPipeline | ExtendableAtlaspackConfigPipeline) {
  return nodes
    .map((node) => (node !== '...' ? getConfigPath(options, node) : null))
    .filter(Boolean);
}

function getConfigPath(options: AtlaspackOptions, node: AtlaspackPluginNode | ExtendableAtlaspackConfigPipeline) {
  return fromProjectPath(options.projectRoot, node.resolveFrom);
}
