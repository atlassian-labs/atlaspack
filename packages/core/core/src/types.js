// @flow strict-local

import type {ContentKey} from '@atlaspack/graph';
import type {
  ASTGenerator,
  BuildMode,
  Engines,
  EnvironmentContext,
  EnvMap,
  FilePath,
  Glob,
  LogLevel,
  Meta,
  DependencySpecifier,
  PackageName,
  ReporterEvent,
  SemverRange,
  ServerOptions,
  SourceType,
  Stats,
  Symbol,
  TargetSourceMapOptions,
  ConfigResult,
  OutputFormat,
  TargetDescriptor,
  HMROptions,
  DetailedReportOptions,
} from '@atlaspack/types';
import type {SharedReference} from '@atlaspack/workers';
import type {FileSystem} from '@atlaspack/fs';
import type {Cache} from '@atlaspack/cache';
import type {PackageManager} from '@atlaspack/package-manager';
import type {ProjectPath} from './projectPath';
import type {Event} from '@parcel/watcher';
import type {FeatureFlags} from '@atlaspack/feature-flags';
import type {BackendType} from '@parcel/watcher';
import type {EnvironmentRef} from './EnvironmentManager';

export type AtlaspackPluginNode = {|
  packageName: PackageName,
  resolveFrom: ProjectPath,
  keyPath?: string,
|};
export type ParcelPluginNode = AtlaspackPluginNode;

export type PureAtlaspackConfigPipeline = $ReadOnlyArray<AtlaspackPluginNode>;
export type ExtendableAtlaspackConfigPipeline = $ReadOnlyArray<
  AtlaspackPluginNode | '...',
>;

export type ProcessedAtlaspackConfig = {|
  resolvers?: PureAtlaspackConfigPipeline,
  transformers?: {[Glob]: ExtendableAtlaspackConfigPipeline, ...},
  bundler: ?ParcelPluginNode,
  namers?: PureAtlaspackConfigPipeline,
  runtimes?: PureAtlaspackConfigPipeline,
  packagers?: {[Glob]: ParcelPluginNode, ...},
  optimizers?: {[Glob]: ExtendableAtlaspackConfigPipeline, ...},
  compressors?: {[Glob]: ExtendableAtlaspackConfigPipeline, ...},
  reporters?: PureAtlaspackConfigPipeline,
  validators?: {[Glob]: ExtendableAtlaspackConfigPipeline, ...},
  filePath: ProjectPath,
  resolveFrom?: ProjectPath,
|};

export type Environment = {|
  id: string,
  context: EnvironmentContext,
  engines: Engines,
  includeNodeModules:
    | boolean
    | Array<PackageName>
    | {[PackageName]: boolean, ...},
  outputFormat: OutputFormat,
  sourceType: SourceType,
  isLibrary: boolean,
  shouldOptimize: boolean,
  shouldScopeHoist: boolean,
  sourceMap: ?TargetSourceMapOptions,
  loc: ?InternalSourceLocation,
  unstableSingleFileOutput: boolean,
|};

export type InternalSourceLocation = {|
  +filePath: ProjectPath,
  /** inclusive */
  +start: {|
    +line: number,
    +column: number,
  |},
  /** exclusive */
  +end: {|
    +line: number,
    +column: number,
  |},
|};

export type Target = {|
  distEntry?: ?FilePath,
  distDir: ProjectPath,
  env: EnvironmentRef,
  name: string,
  publicUrl: string,
  loc?: ?InternalSourceLocation,
  pipeline?: string,
  source?: FilePath | Array<FilePath>,
|};

export const SpecifierType = {
  esm: 0,
  commonjs: 1,
  url: 2,
  custom: 3,
};

export const Priority = {
  sync: 0,
  parallel: 1,
  lazy: 2,
  conditional: 3,
};

// Must match package_json.rs in node-resolver-rs.
export const ExportsCondition = {
  import: 1 << 0,
  require: 1 << 1,
  module: 1 << 2,
  style: 1 << 12,
  sass: 1 << 13,
  less: 1 << 14,
};

export type Dependency = {|
  id: string,
  specifier: DependencySpecifier,
  specifierType: $Values<typeof SpecifierType>,
  priority: $Values<typeof Priority>,
  needsStableName: boolean,
  bundleBehavior: ?$Values<typeof BundleBehavior>,
  isEntry: boolean,
  isOptional: boolean,
  loc: ?InternalSourceLocation,
  env: EnvironmentRef,
  packageConditions?: number,
  customPackageConditions?: Array<string>,
  meta: Meta,
  resolverMeta?: ?Meta,
  resolverPriority?: $Values<typeof Priority>,
  target: ?Target,
  sourceAssetId: ?string,
  sourcePath: ?ProjectPath,
  sourceAssetType?: ?string,
  resolveFrom: ?ProjectPath,
  range: ?SemverRange,
  symbols: ?Map<
    Symbol,
    {|
      local: Symbol,
      loc: ?InternalSourceLocation,
      isWeak: boolean,
      meta?: ?Meta,
    |},
  >,
  pipeline?: ?string,
|};

export const BundleBehavior = {
  inline: 0,
  isolated: 1,
};

export const BundleBehaviorNames: Array<$Keys<typeof BundleBehavior>> =
  Object.keys(BundleBehavior);

export type Asset = {|
  id: ContentKey,
  committed: boolean,
  filePath: ProjectPath,
  query: ?string,
  type: string,
  dependencies: Map<string, Dependency>,
  bundleBehavior: ?$Values<typeof BundleBehavior>,
  isBundleSplittable: boolean,
  isSource: boolean,
  env: EnvironmentRef,
  meta: Meta,
  stats: Stats,
  contentKey: ?string,
  mapKey: ?string,
  outputHash: ?string,
  pipeline: ?string,
  astKey: ?string,
  astGenerator: ?ASTGenerator,
  symbols: ?Map<
    Symbol,
    {|local: Symbol, loc: ?InternalSourceLocation, meta?: ?Meta|},
  >,
  sideEffects: boolean,
  uniqueKey: ?string,
  configPath?: ProjectPath,
  plugin: ?PackageName,
  configKeyPath?: string,
  isLargeBlob?: boolean,
|};

export type InternalGlob = ProjectPath;

export type InternalFile = {|
  +filePath: ProjectPath,
  +hash?: string,
|};

export type FileInvalidation = {|
  type: 'file',
  filePath: ProjectPath,
|};

export type EnvInvalidation = {|
  type: 'env',
  key: string,
|};

export type OptionInvalidation = {|
  type: 'option',
  key: string,
|};

export type RequestInvalidation =
  | FileInvalidation
  | EnvInvalidation
  | OptionInvalidation;

export type InternalFileInvalidation = {|
  filePath: ProjectPath,
|};

export type InternalGlobInvalidation = {|
  glob: InternalGlob,
|};

export type InternalFileAboveInvalidation = {|
  fileName: string,
  aboveFilePath: ProjectPath,
|};

export type InternalFileCreateInvalidation =
  | InternalFileInvalidation
  | InternalGlobInvalidation
  | InternalFileAboveInvalidation;

export type Invalidations = {|
  invalidateOnFileChange: Set<ProjectPath>,
  invalidateOnFileCreate: Array<InternalFileCreateInvalidation>,
  invalidateOnEnvChange: Set<string>,
  invalidateOnOptionChange: Set<string>,
  invalidateOnStartup: boolean,
  invalidateOnBuild: boolean,
|};

export type DevDepRequestRef = {|
  type: 'ref',
  specifier: DependencySpecifier,
  resolveFrom: ProjectPath,
  hash: string,
|};

export type DevDepRequest = {|
  specifier: DependencySpecifier,
  resolveFrom: ProjectPath,
  hash: string,
  invalidateOnFileCreate: Array<InternalFileCreateInvalidation>,
  invalidateOnFileChange: Set<ProjectPath>,
  invalidateOnStartup: boolean,
  additionalInvalidations?: Array<{|
    specifier: DependencySpecifier,
    resolveFrom: ProjectPath,
    range?: ?SemverRange,
  |}>,
|};

declare type GlobPattern = string;

export type AtlaspackOptions = {|
  entries: Array<ProjectPath>,
  config?: DependencySpecifier,
  defaultConfig?: DependencySpecifier,
  env: EnvMap,
  parcelVersion: string,
  targets: ?(Array<string> | {+[string]: TargetDescriptor, ...}),
  shouldDisableCache: boolean,
  cacheDir: FilePath,
  watchDir: FilePath,
  watchIgnore?: Array<FilePath | GlobPattern>,
  watchBackend?: BackendType,
  mode: BuildMode,
  hmrOptions: ?HMROptions,
  shouldContentHash: boolean,
  serveOptions: ServerOptions | false,
  shouldBuildLazily: boolean,
  lazyIncludes: RegExp[],
  lazyExcludes: RegExp[],
  shouldBundleIncrementally: boolean,
  shouldAutoInstall: boolean,
  logLevel: LogLevel,
  projectRoot: FilePath,
  shouldProfile: boolean,
  shouldTrace: boolean,
  shouldPatchConsole: boolean,
  detailedReport?: ?DetailedReportOptions,
  unstableFileInvalidations?: Array<Event>,

  inputFS: FileSystem,
  outputFS: FileSystem,
  cache: Cache,
  packageManager: PackageManager,
  additionalReporters: Array<{|
    packageName: DependencySpecifier,
    resolveFrom: ProjectPath,
  |}>,

  instanceId: string,

  +defaultTargetOptions: {|
    +shouldOptimize: boolean,
    +shouldScopeHoist?: boolean,
    +sourceMaps: boolean,
    +publicUrl: string,
    +distDir?: ProjectPath,
    +engines?: Engines,
    +outputFormat?: OutputFormat,
    +isLibrary?: boolean,
  |},

  +featureFlags: FeatureFlags,
|};
export type ParcelOptions = AtlaspackOptions;

export type AssetNode = {|
  id: ContentKey,
  +type: 'asset',
  value: Asset,
  usedSymbols: Set<Symbol>,
  hasDeferred?: boolean,
  usedSymbolsDownDirty: boolean,
  usedSymbolsUpDirty: boolean,
  requested?: boolean,
|};

export type DependencyNode = {|
  id: ContentKey,
  type: 'dependency',
  value: Dependency,
  complete?: boolean,
  correspondingRequest?: string,
  deferred: boolean,
  /** dependency was deferred (= no used symbols (in immediate parents) & side-effect free) */
  hasDeferred?: boolean,
  usedSymbolsDown: Set<Symbol>,
  /**
   * a requested symbol -> either
   *  - if ambiguous (e.g. dependency to asset group with both CSS modules and JS asset): undefined
   *  - if external: null
   *  - the asset it resolved to, and the potentially renamed export name
   */
  usedSymbolsUp: Map<
    Symbol,
    {|asset: ContentKey, symbol: ?Symbol|} | void | null,
  >,
  /*
   * For the "down" pass, the resolutionAsset needs to be updated.
   * This is set when the AssetGraphBuilder adds/removes/updates nodes.
   */
  usedSymbolsDownDirty: boolean,
  /**
   * In the down pass, `usedSymbolsDown` changed. This needs to be propagated to the resolutionAsset
   * in the up pass.
   */
  usedSymbolsUpDirtyDown: boolean,
  /**
   * In the up pass, `usedSymbolsUp` changed. This needs to be propagated to the sourceAsset in the
   * up pass.
   */
  usedSymbolsUpDirtyUp: boolean,
  /** dependency was excluded (= no used symbols (globally) & side-effect free) */
  excluded: boolean,
|};

export type RootNode = {|id: ContentKey, +type: 'root', value: string | null|};

export type AssetRequestInput = {|
  name?: string, // AssetGraph name, needed so that different graphs can isolated requests since the results are not stored
  filePath: ProjectPath,
  env: EnvironmentRef,
  isSource?: boolean,
  canDefer?: boolean,
  sideEffects?: boolean,
  code?: string,
  pipeline?: ?string,
  optionsRef: SharedReference,
  isURL?: boolean,
  query?: ?string,
  isSingleChangeRebuild?: boolean,
|};

export type AssetRequestResult = Array<Asset>;
// Asset group nodes are essentially used as placeholders for the results of an asset request
export type AssetGroup = $Rest<
  AssetRequestInput,
  {|optionsRef: SharedReference|},
>;
export type AssetGroupNode = {|
  id: ContentKey,
  +type: 'asset_group',
  value: AssetGroup,
  correspondingRequest?: string,
  /** this node was deferred (= no used symbols (in immediate parents) & side-effect free) */
  deferred?: boolean,
  hasDeferred?: boolean,
  usedSymbolsDownDirty: boolean,
|};

export type TransformationRequest = {|
  ...AssetGroup,
  invalidateReason: number,
  devDeps: Map<PackageName, string>,
  invalidDevDeps: Array<{|
    specifier: DependencySpecifier,
    resolveFrom: ProjectPath,
  |}>,
|};

export type DepPathRequestNode = {|
  id: ContentKey,
  +type: 'dep_path_request',
  value: Dependency,
|};

export type AssetRequestNode = {|
  id: ContentKey,
  +type: 'asset_request',
  value: AssetRequestInput,
|};

export type EntrySpecifierNode = {|
  id: ContentKey,
  +type: 'entry_specifier',
  value: ProjectPath,
  correspondingRequest?: string,
|};

export type Entry = {|
  filePath: ProjectPath,
  packagePath: ProjectPath,
  target?: string,
  loc?: ?InternalSourceLocation,
|};

export type EntryFileNode = {|
  id: ContentKey,
  +type: 'entry_file',
  value: Entry,
  correspondingRequest?: string,
|};

export type AssetGraphNode =
  | AssetGroupNode
  | AssetNode
  | DependencyNode
  | EntrySpecifierNode
  | EntryFileNode
  | RootNode;

export type BundleGraphNode =
  | AssetNode
  | DependencyNode
  | EntrySpecifierNode
  | EntryFileNode
  | RootNode
  | BundleGroupNode
  | BundleNode;

export type InternalDevDepOptions = {|
  specifier: DependencySpecifier,
  resolveFrom: ProjectPath,
  range?: ?SemverRange,
  additionalInvalidations?: Array<{|
    specifier: DependencySpecifier,
    resolveFrom: ProjectPath,
    range?: ?SemverRange,
  |}>,
|};

export type Config = {|
  id: string,
  isSource: boolean,
  searchPath: ProjectPath,
  env: EnvironmentRef,
  cacheKey: ?string,
  result: ConfigResult,
  invalidateOnFileChange: Set<ProjectPath>,
  invalidateOnConfigKeyChange: Array<{|
    filePath: ProjectPath,
    configKey: string[],
  |}>,
  invalidateOnFileCreate: Array<InternalFileCreateInvalidation>,
  invalidateOnEnvChange: Set<string>,
  invalidateOnOptionChange: Set<string>,
  devDeps: Array<InternalDevDepOptions>,
  invalidateOnStartup: boolean,
  invalidateOnBuild: boolean,
|};

export type EntryRequest = {|
  specifier: DependencySpecifier,
  result?: ProjectPath,
|};

export type EntryRequestNode = {|
  id: ContentKey,
  +type: 'entry_request',
  value: string,
|};

export type TargetRequestNode = {|
  id: ContentKey,
  +type: 'target_request',
  value: ProjectPath,
|};

export type CacheEntry = {|
  filePath: ProjectPath,
  env: Environment,
  hash: string,
  assets: Array<Asset>,
  // Initial assets, pre-post processing
  initialAssets: ?Array<Asset>,
|};

export type Bundle = {|
  id: ContentKey,
  publicId: ?string,
  hashReference: string,
  type: string,
  env: EnvironmentRef,
  entryAssetIds: Array<ContentKey>,
  mainEntryId: ?ContentKey,
  needsStableName: ?boolean,
  bundleBehavior: ?$Values<typeof BundleBehavior>,
  isSplittable: ?boolean,
  isPlaceholder?: boolean,
  target: Target,
  name: ?string,
  displayName: ?string,
  pipeline: ?string,
  manualSharedBundle?: ?string,
  conditions?: Map<string, string>,
|};

export type BundleNode = {|
  id: ContentKey,
  +type: 'bundle',
  value: Bundle,
|};

export type BundleGroup = {|
  target: Target,
  entryAssetId: string,
|};

export type BundleGroupNode = {|
  id: ContentKey,
  +type: 'bundle_group',
  value: BundleGroup,
|};

export type PackagedBundleInfo = {|
  filePath: ProjectPath,
  bundleId: ContentKey,
  type: string,
  stats: Stats,
|};

export type TransformationOpts = {|
  request: AssetGroup,
  optionsRef: SharedReference,
  configCachePath: string,
|};

export type ValidationOpts = {|
  requests: AssetGroup[],
  optionsRef: SharedReference,
  configCachePath: string,
|};

export type ReportFn = (event: ReporterEvent) => void | Promise<void>;

export type Condition = {|
  publicId: string,
  assets: Set<Asset>,
  key: string,
  ifTrueDependency: Dependency,
  ifFalseDependency: Dependency,
|};
