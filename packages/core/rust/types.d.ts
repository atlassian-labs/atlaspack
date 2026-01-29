/* tslint:disable */
/* eslint-disable */

export type AtlaspackNapi = any;
export type JsTransferable = any;
export type NapiSideEffectsVariants = any;
export type LMDBOptions = any;
export type SpanId = any;
export type BundleGraphNode = any;
export interface LmdbOptions {
  /** The database directory path */
  path: string;
  /**
   * If enabled, the database writer will set the following flags:
   *
   * * MAP_ASYNC - "use asynchronous msync when MDB_WRITEMAP is used"
   * * NO_SYNC - "don't fsync after commit"
   * * NO_META_SYNC - "don't fsync metapage after commit"
   *
   * `MDB_WRITEMAP` is on by default.
   */
  asyncWrites: boolean;
  /**
   * The mmap size, this corresponds to [`mdb_env_set_mapsize`](http://www.lmdb.tech/doc/group__mdb.html#gaa2506ec8dab3d969b0e609cd82e619e5)
   * if this isn't set it'll default to around 10MB.
   */
  mapSize?: number;
}
export declare function initTracingSubscriber(): void;
export interface Entry {
  key: string;
  value: Buffer;
}
export declare function findAncestorFile(
  filenames: Array<string>,
  from: string,
  root: string,
): string | null;
export declare function findFirstFile(names: Array<string>): string | null;
export declare function findNodeModule(
  module: string,
  from: string,
): string | null;
export declare function hashString(s: string): string;
export declare function hashBuffer(buf: Buffer): string;
export declare function optimizeImage(kind: string, buf: Buffer): Buffer;
export declare function createAssetId(params: unknown): string;
export interface AtlaspackNapiOptions {
  fs?: object;
  options: object;
  packageManager?: object;
  threads?: number;
  napiWorkerPool: object;
}
export declare function atlaspackNapiCreate(
  napiOptions: AtlaspackNapiOptions,
  lmdb: LMDB,
): object;
export declare function atlaspackNapiBuildAssetGraph(
  atlaspackNapi: AtlaspackNapi,
): object;
export declare function atlaspackNapiRespondToFsEvents(
  atlaspackNapi: AtlaspackNapi,
  options: object,
): object;
export interface CacheStats {
  hits: number;
  misses: number;
  uncacheables: number;
  bailouts: number;
  errors: number;
  validations: number;
}
export declare function atlaspackNapiCompleteSession(
  atlaspackNapi: AtlaspackNapi,
): Promise<CacheStats>;
export declare function createDependencyId(params: unknown): string;
export declare function createEnvironmentId(params: unknown): string;
/** Overwrite all environments with a new set of environments */
export declare function setAllEnvironments(environments: unknown): void;
/** Get an array of all environments */
export declare function getAllEnvironments(): Array<unknown>;
/** Get environment by ID */
export declare function getEnvironment(id: string): unknown;
/** Add an environment to the global manager */
export declare function addEnvironment(environment: unknown): void;
export declare function getAvailableThreads(): number;
export declare function initializeMonitoring(): void;
export declare function closeMonitoring(): void;
export declare function getNativeMemoryStats(): NativeMemoryStats | null;
export declare function resetMemoryTracking(): void;
export declare function sampleNativeMemory(): void;
/** Called on the worker thread to create a reference to the NodeJs worker */
export declare function newNodejsWorker(worker: object): JsTransferable;
export interface InlineRequiresOptimizerInput {
  code: string;
  sourceMaps: boolean;
  ignoreModuleIds: Array<string>;
}
export interface InlineRequiresOptimizerResult {
  code: string;
  sourceMap?: string;
}
export declare function runInlineRequiresOptimizer(
  input: InlineRequiresOptimizerInput,
): InlineRequiresOptimizerResult;
/** Runs in the rayon thread pool */
export declare function runInlineRequiresOptimizerAsync(
  input: InlineRequiresOptimizerInput,
): object;
export interface JsFileSystemOptions {
  canonicalize: (...args: any[]) => any;
  read: (...args: any[]) => any;
  isFile: (...args: any[]) => any;
  isDir: (...args: any[]) => any;
  includeNodeModules?: NapiSideEffectsVariants;
}
export interface FileSystem {
  fs?: JsFileSystemOptions;
  includeNodeModules?: NapiSideEffectsVariants;
  conditions?: number;
  moduleDirResolver?: (...args: any[]) => any;
  mode: number;
  entries?: number;
  extensions?: Array<string>;
  packageExports: boolean;
  typescript?: boolean;
  reduceStringCreation?: boolean;
}
export interface ResolveOptions {
  filename: string;
  specifierType: string;
  parent: string;
  packageConditions?: Array<string>;
}
export interface FilePathCreateInvalidation {
  filePath: string;
}
export interface FileNameCreateInvalidation {
  fileName: string;
  aboveFilePath: string;
}
export interface GlobCreateInvalidation {
  glob: string;
}
export interface ResolveResult {
  resolution: unknown;
  invalidateOnFileChange: Array<string>;
  invalidateOnFileCreate: Array<
    | FilePathCreateInvalidation
    | FileNameCreateInvalidation
    | GlobCreateInvalidation
  >;
  query?: string;
  sideEffects: boolean;
  error: unknown;
  moduleType: number;
}
export interface JsInvalidations {
  invalidateOnFileChange: Array<string>;
  invalidateOnFileCreate: Array<
    | FilePathCreateInvalidation
    | FileNameCreateInvalidation
    | GlobCreateInvalidation
  >;
  invalidateOnStartup: boolean;
}
export interface Replacement {
  from: string;
  to: string;
}
export declare function performStringReplacements(
  input: string,
  replacements: Array<Replacement>,
): string;
export declare function transform(opts: object): unknown;
export declare function transformAsync(opts: object): object;
export declare function determineJsxConfiguration(
  file_path: string,
  is_source: boolean,
  config: JsObject | null | undefined,
  project_root: string,
): object;
export declare function getVcsStateSnapshot(
  path: string,
  excludePatterns: Array<string>,
): object;
export declare function getEventsSince(
  repoPath: string,
  vcsStateSnapshot: unknown,
  newRev?: string | undefined | null,
): object;
export type LMDB = Lmdb;
export class Lmdb {
  constructor(options: LmdbOptions);
  get(key: string): Promise<Buffer | null | undefined>;
  hasSync(key: string): boolean;
  keysSync(skip: number, limit: number): Array<string>;
  getSync(key: string): Buffer | null;
  getManySync(keys: Array<string>): Array<Buffer | undefined | null>;
  putMany(entries: Array<Entry>): Promise<void>;
  put(key: string, data: Buffer): Promise<void>;
  putNoConfirm(key: string, data: Buffer): void;
  delete(key: string): Promise<void>;
  startReadTransaction(): void;
  commitReadTransaction(): void;
  startWriteTransaction(): Promise<void>;
  commitWriteTransaction(): Promise<void>;
  /** Compact the database to the target path */
  compact(targetPath: string): void;
  constructor(options: LMDBOptions);
  get(key: string): Promise<Buffer | null | undefined>;
  hasSync(key: string): boolean;
  keysSync(skip: number, limit: number): Array<string>;
  getSync(key: string): Buffer | null;
  getManySync(keys: Array<string>): Array<Buffer | undefined | null>;
  putMany(entries: Array<Entry>): Promise<void>;
  put(key: string, data: Buffer): Promise<void>;
  delete(key: string): Promise<void>;
  putNoConfirm(key: string, data: Buffer): void;
  startReadTransaction(): void;
  commitReadTransaction(): void;
  startWriteTransaction(): Promise<void>;
  commitWriteTransaction(): Promise<void>;
  compact(targetPath: string): void;
}
export class Hash {
  constructor();
  writeString(s: string): void;
  writeBuffer(buf: Buffer): void;
  finish(): string;
}
export class AtlaspackTracer {
  constructor();
  enter(label: string): SpanId;
  exit(id: SpanId): void;
}
export class Resolver {
  constructor(projectRoot: string, options: FileSystem);
  resolve(options: ResolveOptions): ResolveResult;
  resolveAsync(): object;
  resolveAsync(options: ResolveOptions): object;
  getInvalidations(path: string): JsInvalidations;
  getInvalidations(path: string): JsInvalidations;
}

export interface SourceLocation {
  filePath: string;
  start: {
    line: number;
    column: number;
    line: number;
    column: number;
  };
  end: {
    line: number;
    column: number;
    line: number;
    column: number;
  };
}

export interface Symbol {
  local: string;
  exported: string;
  loc?: SourceLocation;
  isWeak: boolean;
  isEsmExport: boolean;
  isStaticBindingSafe: boolean;
  selfReferenced: boolean;
}

export interface TokensPluginOptions {
  tokenDataPath: string;
  shouldUseAutoFallback: boolean;
  shouldForceAutoFallback: boolean;
  forceAutoFallbackExemptions: Array<string>;
  defaultTheme: string;
}

export interface TokensConfig {
  filename: string;
  projectRoot: string;
  isSource: boolean;
  sourceMaps: boolean;
  tokensOptions: TokensPluginOptions;
}

export interface TokensJsSourceLocation {
  start_line: number;
  start_col: number;
  end_line: number;
  end_col: number;
}

export interface TokensJsCodeHighlight {
  message: string | null;
  loc: TokensJsSourceLocation;
}

export interface TokensJsDiagnostic {
  message: string;
  code_highlights: Array<TokensJsCodeHighlight> | null;
  hints: Array<string> | null;
  show_environment: boolean;
  severity: string;
  documentation_url: string | null;
}

export interface TokensPluginResult {
  code: string;
  map: string | null;
  diagnostics: Array<TokensJsDiagnostic>;
}

/** Apply the tokens transformation plugin to the given code asynchronously */
export declare function applyTokensPlugin(
  rawCode: Buffer,
  config: TokensConfig,
): object;

export interface DetailedMemoryStats {
  min: number;
  max: number;
  mean: number;
  median: number;
  p95: number;
  p99: number;
  standardDeviation: number;
  range: number;
}

export interface NativeMemoryStats {
  physicalMem: DetailedMemoryStats;
  virtualMem: DetailedMemoryStats;
  sampleCount: number;
}

export type JsSourceMap = SourceMap;
export class SourceMap {
  constructor(projectRoot: string, buffer?: Buffer | undefined | null);
  addSource(source: string): number;
  getSource(sourceIndex: number): string;
  getSources(): Array<string>;
  getSourcesContent(): Array<string>;
  getSourceIndex(source: string): number;
  setSourceContentBySource(source: string, sourceContent: string): void;
  getSourceContentBySource(source: string): string;
  addName(name: string): number;
  getName(nameIndex: number): string;
  getNames(): Array<string>;
  getNameIndex(name: string): number;
  getMappings(): unknown[];
  toBuffer(): Buffer;
  addSourceMap(sourcemapObject: SourceMap, lineOffset: number): void;
  addVLQMap(
    vlqMappings: string,
    sources: Array<string>,
    sourcesContent: Array<string>,
    names: Array<string>,
    lineOffset: number,
    columnOffset: number,
  ): void;
  toVLQ(): object;
  addIndexedMappings(mappings: JsTypedArray): void;
  offsetLines(generatedLine: number, generatedLineOffset: number): void;
  offsetColumns(
    generatedLine: number,
    generatedColumn: number,
    generatedColumnOffset: number,
  ): void;
  addEmptyMap(source: string, sourceContent: string, lineOffset: number): void;
  extends(originalSourcemap: SourceMap): void;
  findClosestMapping(
    generatedLine: number,
    generatedColumn: number,
  ): object | null;
  getProjectRoot(): string;
}

export declare function atlaspackNapiPackage(
  atlaspackNapi: AtlaspackNapi,
): object;
export interface CompiledCssInJsConfigPlugin {
  configPath?: string;
  importReact?: boolean;
  nonce?: string;
  importSources?: Array<string>;
  optimizeCss?: boolean;
  extensions?: Array<string>;
  addComponentName?: boolean;
  processXcss?: boolean;
  increaseSpecificity?: boolean;
  sortAtRules?: boolean;
  sortShorthand?: boolean;
  classHashPrefix?: string;
  flattenMultipleSelectors?: boolean;
  extract?: boolean;
  ssr?: boolean;
  unsafeReportSafeAssetsForMigration?: boolean;
  unsafeUseSafeAssets?: boolean;
  unsafeSkipPattern?: string;
}
export declare function hashCode(rawCode: string): string;
export declare function isSafeFromJs(hash: string, configPath: string): boolean;
export declare function applyCompiledCssInJsPlugin(
  rawCode: Buffer,
  input: CompiledCssInJsPluginInput,
): object;
export interface CompiledCssInJsPluginResult {
  code: string;
  map?: string;
  styleRules: Array<string>;
  diagnostics: Array<JsDiagnostic>;
  bailOut: boolean;
}
export declare function atlaspackNapiLoadBundleGraph(
  atlaspackNapi: AtlaspackNapi,
  nodes: string,
  edges: Array<[number, number, number]>,
): object;
