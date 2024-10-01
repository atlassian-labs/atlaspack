// @ts-expect-error - TS2307 - Cannot find module 'flow-to-typescript-codemod' or its corresponding type declarations.
import {Flow} from 'flow-to-typescript-codemod';

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
// @ts-expect-error - TS2614 - Module '"@atlaspack/workers"' has no exported member 'SharedReference'. Did you mean to use 'import SharedReference from "@atlaspack/workers"' instead?
import type {SharedReference} from '@atlaspack/workers';
import type {FileSystem} from '@atlaspack/fs';
import type {Cache} from '@atlaspack/cache';
import type {PackageManager} from '@atlaspack/package-manager';
import type {ProjectPath} from './projectPath';
import type {Event} from '@parcel/watcher';
import type {FeatureFlags} from '@atlaspack/feature-flags';
import type {BackendType} from '@parcel/watcher';

export type AtlaspackPluginNode = {
  packageName: PackageName;
  resolveFrom: ProjectPath;
  keyPath?: string;
};
export type ParcelPluginNode = AtlaspackPluginNode;

export type PureAtlaspackConfigPipeline = ReadonlyArray<AtlaspackPluginNode>;
export type ExtendableAtlaspackConfigPipeline = ReadonlyArray<
  AtlaspackPluginNode | '...'
>;

export type ProcessedAtlaspackConfig = {
  resolvers?: PureAtlaspackConfigPipeline;
  transformers?: Partial<Record<Glob, ExtendableAtlaspackConfigPipeline>>;
  bundler: ParcelPluginNode | null | undefined;
  namers?: PureAtlaspackConfigPipeline;
  runtimes?: PureAtlaspackConfigPipeline;
  packagers?: Partial<Record<Glob, ParcelPluginNode>>;
  optimizers?: Partial<Record<Glob, ExtendableAtlaspackConfigPipeline>>;
  compressors?: Partial<Record<Glob, ExtendableAtlaspackConfigPipeline>>;
  reporters?: PureAtlaspackConfigPipeline;
  validators?: Partial<Record<Glob, ExtendableAtlaspackConfigPipeline>>;
  filePath: ProjectPath;
  resolveFrom?: ProjectPath;
};

export type Environment = {
  id: string;
  context: EnvironmentContext;
  engines: Engines;
  includeNodeModules:
    | boolean
    | Array<PackageName>
    | Partial<Record<PackageName, boolean>>;
  outputFormat: OutputFormat;
  sourceType: SourceType;
  isLibrary: boolean;
  shouldOptimize: boolean;
  shouldScopeHoist: boolean;
  sourceMap: TargetSourceMapOptions | null | undefined;
  loc: InternalSourceLocation | null | undefined;
};

export type InternalSourceLocation = {
  readonly filePath: ProjectPath;
  /** inclusive */
  readonly start: {
    readonly line: number;
    readonly column: number;
  };
  /** exclusive */
  readonly end: {
    readonly line: number;
    readonly column: number;
  };
};

export type Target = {
  distEntry?: FilePath | null | undefined;
  distDir: ProjectPath;
  env: Environment;
  name: string;
  publicUrl: string;
  loc?: InternalSourceLocation | null | undefined;
  pipeline?: string;
  source?: FilePath | Array<FilePath>;
};

export const SpecifierType = {
  esm: 0,
  commonjs: 1,
  url: 2,
  custom: 3,
} as const;

export const Priority = {
  sync: 0,
  parallel: 1,
  lazy: 2,
} as const;

// Must match package_json.rs in node-resolver-rs.
export const ExportsCondition = {
  import: 1 << 0,
  require: 1 << 1,
  module: 1 << 2,
  style: 1 << 12,
  sass: 1 << 13,
  less: 1 << 14,
} as const;

export type Dependency = {
  id: string;
  specifier: DependencySpecifier;
  specifierType: typeof SpecifierType[keyof typeof SpecifierType];
  priority: typeof Priority[keyof typeof Priority];
  needsStableName: boolean;
  bundleBehavior:
    | typeof BundleBehavior[keyof typeof BundleBehavior]
    | null
    | undefined;
  isEntry: boolean;
  isOptional: boolean;
  loc: InternalSourceLocation | null | undefined;
  env: Environment;
  packageConditions?: number;
  customPackageConditions?: Array<string>;
  meta: Meta;
  resolverMeta?: Meta | null | undefined;
  target: Target | null | undefined;
  sourceAssetId: string | null | undefined;
  sourcePath: ProjectPath | null | undefined;
  sourceAssetType?: string | null | undefined;
  resolveFrom: ProjectPath | null | undefined;
  range: SemverRange | null | undefined;
  symbols:
    | Map<
        symbol,
        {
          local: symbol;
          loc: InternalSourceLocation | null | undefined;
          isWeak: boolean;
          meta?: Meta | null | undefined;
        }
      >
    | null
    | undefined;
  pipeline?: string | null | undefined;
};

export const BundleBehavior = {
  inline: 0,
  isolated: 1,
} as const;

// @ts-expect-error - TS2322 - Type 'string[]' is not assignable to type '("inline" | "isolated")[]'.
export const BundleBehaviorNames: Array<keyof typeof BundleBehavior> =
  Object.keys(BundleBehavior);

export type Asset = {
  id: ContentKey;
  committed: boolean;
  filePath: ProjectPath;
  query: string | null | undefined;
  type: string;
  dependencies: Map<string, Dependency>;
  bundleBehavior:
    | typeof BundleBehavior[keyof typeof BundleBehavior]
    | null
    | undefined;
  isBundleSplittable: boolean;
  isSource: boolean;
  env: Environment;
  meta: Meta;
  stats: Stats;
  contentKey: string | null | undefined;
  mapKey: string | null | undefined;
  outputHash: string | null | undefined;
  pipeline: string | null | undefined;
  astKey: string | null | undefined;
  astGenerator: ASTGenerator | null | undefined;
  symbols:
    | Map<
        symbol,
        {
          local: symbol;
          loc: InternalSourceLocation | null | undefined;
          meta?: Meta | null | undefined;
        }
      >
    | null
    | undefined;
  sideEffects: boolean;
  uniqueKey: string | null | undefined;
  configPath?: ProjectPath;
  plugin: PackageName | null | undefined;
  configKeyPath?: string;
  isLargeBlob?: boolean;
};

export type InternalGlob = ProjectPath;

export type InternalFile = {
  readonly filePath: ProjectPath;
  readonly hash?: string;
};

export type FileInvalidation = {
  type: 'file';
  filePath: ProjectPath;
};

export type EnvInvalidation = {
  type: 'env';
  key: string;
};

export type OptionInvalidation = {
  type: 'option';
  key: string;
};

export type RequestInvalidation =
  | FileInvalidation
  | EnvInvalidation
  | OptionInvalidation;

export type InternalFileInvalidation = {
  filePath: ProjectPath;
};

export type InternalGlobInvalidation = {
  glob: InternalGlob;
};

export type InternalFileAboveInvalidation = {
  fileName: string;
  aboveFilePath: ProjectPath;
};

export type InternalFileCreateInvalidation =
  | InternalFileInvalidation
  | InternalGlobInvalidation
  | InternalFileAboveInvalidation;

export type Invalidations = {
  invalidateOnFileChange: Set<ProjectPath>;
  invalidateOnFileCreate: Array<InternalFileCreateInvalidation>;
  invalidateOnEnvChange: Set<string>;
  invalidateOnOptionChange: Set<string>;
  invalidateOnStartup: boolean;
  invalidateOnBuild: boolean;
};

export type DevDepRequest = {
  specifier: DependencySpecifier;
  resolveFrom: ProjectPath;
  hash: string;
  invalidateOnFileCreate?: Array<InternalFileCreateInvalidation>;
  invalidateOnFileChange?: Set<ProjectPath>;
  invalidateOnStartup?: boolean;
  additionalInvalidations?: Array<{
    specifier: DependencySpecifier;
    resolveFrom: ProjectPath;
    range?: SemverRange | null | undefined;
  }>;
};

declare type GlobPattern = string;

export type AtlaspackOptions = {
  entries: Array<ProjectPath>;
  config?: DependencySpecifier;
  defaultConfig?: DependencySpecifier;
  env: EnvMap;
  parcelVersion: string;
  targets:
    | Array<string>
    | {
        readonly [key: string]: TargetDescriptor;
      }
    | null
    | undefined;
  shouldDisableCache: boolean;
  cacheDir: FilePath;
  watchDir: FilePath;
  watchIgnore?: Array<FilePath | GlobPattern>;
  watchBackend?: BackendType;
  mode: BuildMode;
  hmrOptions: HMROptions | null | undefined;
  shouldContentHash: boolean;
  serveOptions: ServerOptions | false;
  shouldBuildLazily: boolean;
  lazyIncludes: RegExp[];
  lazyExcludes: RegExp[];
  shouldBundleIncrementally: boolean;
  shouldAutoInstall: boolean;
  logLevel: LogLevel;
  projectRoot: FilePath;
  shouldProfile: boolean;
  shouldTrace: boolean;
  shouldPatchConsole: boolean;
  detailedReport?: DetailedReportOptions | null | undefined;
  unstableFileInvalidations?: Array<Event>;
  inputFS: FileSystem;
  outputFS: FileSystem;
  cache: Cache;
  packageManager: PackageManager;
  additionalReporters: Array<{
    packageName: DependencySpecifier;
    resolveFrom: ProjectPath;
  }>;
  instanceId: string;
  readonly defaultTargetOptions: {
    readonly shouldOptimize: boolean;
    readonly shouldScopeHoist?: boolean;
    readonly sourceMaps: boolean;
    readonly publicUrl: string;
    readonly distDir?: ProjectPath;
    readonly engines?: Engines;
    readonly outputFormat?: OutputFormat;
    readonly isLibrary?: boolean;
  };
  readonly featureFlags: FeatureFlags;
};
export type ParcelOptions = AtlaspackOptions;

export type AssetNode = {
  id: ContentKey;
  readonly type: 'asset';
  value: Asset;
  usedSymbols: Set<symbol>;
  hasDeferred?: boolean;
  usedSymbolsDownDirty: boolean;
  usedSymbolsUpDirty: boolean;
  requested?: boolean;
};

export type DependencyNode = {
  id: ContentKey;
  type: 'dependency';
  value: Dependency;
  complete?: boolean;
  correspondingRequest?: string;
  deferred: boolean;
  /** dependency was deferred (= no used symbols (in immediate parents) & side-effect free) */
  hasDeferred?: boolean;
  usedSymbolsDown: Set<symbol>;
  /**
   * a requested symbol -> either
   *  - if ambiguous (e.g. dependency to asset group with both CSS modules and JS asset): undefined
   *  - if external: null
   *  - the asset it resolved to, and the potentially renamed export name
   */
  usedSymbolsUp: Map<
    symbol,
    | {
        asset: ContentKey;
        symbol: symbol | null | undefined;
      }
    | undefined
    | null
  >;
  /*
   * For the "down" pass, the resolutionAsset needs to be updated.
   * This is set when the AssetGraphBuilder adds/removes/updates nodes.
   */
  usedSymbolsDownDirty: boolean;
  /**
   * In the down pass, `usedSymbolsDown` changed. This needs to be propagated to the resolutionAsset
   * in the up pass.
   */
  usedSymbolsUpDirtyDown: boolean;
  /**
   * In the up pass, `usedSymbolsUp` changed. This needs to be propagated to the sourceAsset in the
   * up pass.
   */
  usedSymbolsUpDirtyUp: boolean;
  /** dependency was excluded (= no used symbols (globally) & side-effect free) */
  excluded: boolean;
};

export type RootNode = {
  id: ContentKey;
  readonly type: 'root';
  value: string | null;
};

export type AssetRequestInput = {
  name?: string; // AssetGraph name, needed so that different graphs can isolated requests since the results are not stored,
  filePath: ProjectPath;
  env: Environment;
  isSource?: boolean;
  canDefer?: boolean;
  sideEffects?: boolean;
  code?: string;
  pipeline?: string | null | undefined;
  optionsRef: SharedReference;
  isURL?: boolean;
  query?: string | null | undefined;
  isSingleChangeRebuild?: boolean;
};

export type AssetRequestResult = Array<Asset>;
// Asset group nodes are essentially used as placeholders for the results of an asset request
export type AssetGroup = Partial<
  Flow.Diff<
    AssetRequestInput,
    {
      optionsRef: SharedReference;
    }
  >
>;
export type AssetGroupNode = {
  id: ContentKey;
  readonly type: 'asset_group';
  value: AssetGroup;
  correspondingRequest?: string;
  /** this node was deferred (= no used symbols (in immediate parents) & side-effect free) */
  deferred?: boolean;
  hasDeferred?: boolean;
  usedSymbolsDownDirty: boolean;
};

export type TransformationRequest = AssetGroup & {
  invalidateReason: number;
  devDeps: Map<PackageName, string>;
  invalidDevDeps: Array<{
    specifier: DependencySpecifier;
    resolveFrom: ProjectPath;
  }>;
};

export type DepPathRequestNode = {
  id: ContentKey;
  readonly type: 'dep_path_request';
  value: Dependency;
};

export type AssetRequestNode = {
  id: ContentKey;
  readonly type: 'asset_request';
  value: AssetRequestInput;
};

export type EntrySpecifierNode = {
  id: ContentKey;
  readonly type: 'entry_specifier';
  value: ProjectPath;
  correspondingRequest?: string;
};

export type Entry = {
  filePath: ProjectPath;
  packagePath: ProjectPath;
  target?: string;
  loc?: InternalSourceLocation | null | undefined;
};

export type EntryFileNode = {
  id: ContentKey;
  readonly type: 'entry_file';
  value: Entry;
  correspondingRequest?: string;
};

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

export type InternalDevDepOptions = {
  specifier: DependencySpecifier;
  resolveFrom: ProjectPath;
  range?: SemverRange | null | undefined;
  additionalInvalidations?: Array<{
    specifier: DependencySpecifier;
    resolveFrom: ProjectPath;
    range?: SemverRange | null | undefined;
  }>;
};

export type Config = {
  id: string;
  isSource: boolean;
  searchPath: ProjectPath;
  env: Environment;
  cacheKey: string | null | undefined;
  result: ConfigResult;
  invalidateOnFileChange: Set<ProjectPath>;
  invalidateOnConfigKeyChange: Array<{
    filePath: ProjectPath;
    configKey: string;
  }>;
  invalidateOnFileCreate: Array<InternalFileCreateInvalidation>;
  invalidateOnEnvChange: Set<string>;
  invalidateOnOptionChange: Set<string>;
  devDeps: Array<InternalDevDepOptions>;
  invalidateOnStartup: boolean;
  invalidateOnBuild: boolean;
};

export type EntryRequest = {
  specifier: DependencySpecifier;
  result?: ProjectPath;
};

export type EntryRequestNode = {
  id: ContentKey;
  readonly type: 'entry_request';
  value: string;
};

export type TargetRequestNode = {
  id: ContentKey;
  readonly type: 'target_request';
  value: ProjectPath;
};

export type CacheEntry = {
  filePath: ProjectPath;
  env: Environment;
  hash: string;
  assets: Array<Asset>;
  // Initial assets, pre-post processing
  initialAssets: Array<Asset> | null | undefined;
};

export type Bundle = {
  id: ContentKey;
  publicId: string | null | undefined;
  hashReference: string;
  type: string;
  env: Environment;
  entryAssetIds: Array<ContentKey>;
  mainEntryId: ContentKey | null | undefined;
  needsStableName: boolean | null | undefined;
  bundleBehavior:
    | typeof BundleBehavior[keyof typeof BundleBehavior]
    | null
    | undefined;
  isSplittable: boolean | null | undefined;
  isPlaceholder?: boolean;
  target: Target;
  name: string | null | undefined;
  displayName: string | null | undefined;
  pipeline: string | null | undefined;
  manualSharedBundle?: string | null | undefined;
};

export type BundleNode = {
  id: ContentKey;
  readonly type: 'bundle';
  value: Bundle;
};

export type BundleGroup = {
  target: Target;
  entryAssetId: string;
};

export type BundleGroupNode = {
  id: ContentKey;
  readonly type: 'bundle_group';
  value: BundleGroup;
};

export type PackagedBundleInfo = {
  filePath: ProjectPath;
  type: string;
  stats: Stats;
};

export type TransformationOpts = {
  request: AssetGroup;
  optionsRef: SharedReference;
  configCachePath: string;
};

export type ValidationOpts = {
  requests: AssetGroup[];
  optionsRef: SharedReference;
  configCachePath: string;
};

export type ReportFn = (event: ReporterEvent) => undefined | Promise<undefined>;
