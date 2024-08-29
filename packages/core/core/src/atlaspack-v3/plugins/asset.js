/* eslint-disable no-unused-vars */
// @flow

import type {
  Environment,
  EnvironmentOptions,
  DependencyOptions,
  Dependency,
  AST,
  Meta,
  ASTGenerator,
  FileSystem,
  FilePath,
  BundleBehavior,
  AssetSymbols,
  FileCreateInvalidation,
} from '@atlaspack/types';
import type SourceMap from '@parcel/source-map';
import {Readable} from 'stream';

export type NapiTransformResult = {|
  asset: NapiAsset,
  dependencies: Array<NapiDependency>,
  invalidateOnFileChange: Array<string>,
|};

export type NapiAsset = {|
  id: any, //number,
  bundleBehavior: number, //BundleBehavior,
  env: any, //Arc<Environment>,
  filePath: any, //PathBuf,
  type: any, //FileType,
  code: any, //Arc<Code>,
  meta: any, //JSONObject,
  pipeline: any, //Option<String>,
  query: any, //Option<String>,
  stats: any, //AssetStats,
  symbols: any, //Vec<Symbol>,
  uniqueKey: any, //Option<String>,
  sideEffects: boolean,
  isBundleSplittable: any, //bool,
  isSource: any, //bool,
  hasCjsExports: any, //bool,
  staticExports: any, //bool,
  shouldWrap: boolean,
  hasNodeReplacements: any, //bool,
  isConstantModule: any, //bool,
  hasSymbols: any, //bool,
|};

export type NapiDependency = {|
  bundleBehavior: any, //BundleBehavior,
  env: any, //Arc<Environment>,
  loc: any, //Option<SourceLocation>,
  meta: any, //JSONObject,
  packageConditions: any, //ExportsCondition,
  pipeline: any, //Option<String>,
  priority: any, //Priority,
  range: any, //Option<String>,
  resolveFrom: any, //Option<PathBuf>,
  sourceAssetId: any, //Option<String>,
  sourcePath: any, //Option<PathBuf>,
  specifier: any, //String,
  specifierType: any, //SpecifierType,
  symbols: any, //Vec<Symbol>,
  target: any, //Option<Box<Target>>,
  isEntry: any, //bool,
  isOptional: any, //bool,
  needsStableName: any, //bool,
  shouldWrap: any, //bool,
  isEsm: any, //bool,
  hasSymbols: any, //bool,
  placeholder: any, //Option<String>,
|};

export class MappedNapiAsset {
  #internal: NapiAsset;
  #query: URLSearchParams | null;
  #dependencies: Array<NapiDependency>;

  get id(): string {
    return this.#internal.id;
  }

  get fs(): FileSystem {
    // $FlowFixMe
    return {};
  }

  get filePath(): FilePath {
    return this.#internal.filePath;
  }

  get type(): string {
    return this.#internal.type;
  }

  set type(v: string) {
    this.#internal.type = v;
  }

  get query(): URLSearchParams {
    if (!this.#query) {
      this.#query = new URLSearchParams(this.#internal.query);
    }
    return this.#query;
  }

  get env(): Environment {
    // $FlowFixMe
    return this.#internal.env;
  }

  get isSource(): boolean {
    return this.#internal.isSource;
  }

  get meta(): Meta {
    return this.#internal.meta;
  }

  get bundleBehavior(): ?BundleBehavior {
    return bundleBehaviorFromNapi(this.#internal.bundleBehavior);
  }

  set bundleBehavior(bundleBehavior: ?BundleBehavior) {
    this.#internal.bundleBehavior = bundleBehaviorToNapi(bundleBehavior);
  }

  get isBundleSplittable(): boolean {
    return this.#internal.isBundleSplittable;
  }

  set isBundleSplittable(v: boolean) {
    this.#internal.isBundleSplittable = v;
  }

  get sideEffects(): boolean {
    return this.#internal.sideEffects;
  }

  set sideEffects(v: boolean) {
    this.#internal.sideEffects = v;
  }

  get uniqueKey(): ?string {
    return this.#internal.uniqueKey;
  }

  set uniqueKey(v: ?string) {
    this.#internal.uniqueKey = v;
  }

  get astGenerator(): ?ASTGenerator {
    return undefined;
  }

  get pipeline(): ?string {
    return this.#internal.pipeline;
  }

  get symbols(): AssetSymbols {
    return this.#internal.symbols;
  }

  constructor(internal: NapiAsset, dependencies: Array<NapiDependency>) {
    this.#internal = internal;
    this.#dependencies = dependencies;
    this.#query = null;
  }

  addDependency(options: DependencyOptions): string {
    this.#dependencies.push({
      bundleBehavior: bundleBehaviorToNapi(options.bundleBehavior),
      env: options.env || this.#internal.env, // todo
      loc: options.loc,
      meta: options.meta,
      packageConditions: options.packageConditions,
      pipeline: options.pipeline,
      priority: options.priority,
      range: options.range,
      resolveFrom: options.resolveFrom,
      sourceAssetId: this.#internal.id,
      sourcePath: this.#internal.filePath,
      specifier: options.specifier,
      specifierType: options.specifierType,
      symbols: options.symbols,
      target: undefined, // todo
      isEntry: false, // todo
      isOptional: options.isOptional,
      needsStableName: options.needsStableName,
      shouldWrap: false, // todo
      isEsm: options.specifierType === 'esm', //todo
      hasSymbols: false, // todo
      placeholder: undefined, // todo
    });
    return ''; // todo
  }

  addURLDependency(url: string, opts: $Shape<DependencyOptions>): string {
    return '';
  }

  invalidateOnFileChange(filepath: FilePath): void {}

  invalidateOnFileCreate(
    fileCreateInvalidation: FileCreateInvalidation,
  ): void {}

  invalidateOnEnvChange(v: string): void {}

  invalidateOnStartup(): void {}

  invalidateOnBuild(): void {}

  setCode(code: string): void {}

  setBuffer(buf: Buffer): void {}

  setStream(stream: Readable): void {}

  setAST(ast: AST): void {}

  isASTDirty(): boolean {
    return true;
  }

  setMap(sourcemap: ?SourceMap): void {}

  setEnvironment(opts: EnvironmentOptions): void {}

  getAST(): Promise<?AST> {
    return Promise.resolve(undefined);
  }

  getCode(): Promise<string> {
    return Promise.resolve(this.#internal.code);
  }

  async getBuffer(): Promise<Buffer> {
    return Buffer.from(await this.getCode());
  }

  getStream(): Readable {
    return Readable.from(this.#internal.code);
  }

  getMap(): Promise<?SourceMap> {
    return Promise.resolve(undefined);
  }

  getMapBuffer(): Promise<?Buffer> {
    return Promise.resolve(undefined);
  }

  getDependencies(): $ReadOnlyArray<Dependency> {
    return [];
  }
}

function bundleBehaviorToNapi(input: ?BundleBehavior): number {
  switch (input) {
    case 'inline':
      return 0;
    case 'isolated':
      return 1;
    case undefined:
      return 255;
    default:
      throw new Error('Invalid BundleBehavior');
  }
}

function bundleBehaviorFromNapi(input: number): ?BundleBehavior {
  switch (input) {
    case 0:
      return 'inline';
    case 1:
      return 'isolated';
    case 255:
      return undefined;
    default:
      throw new Error('Invalid BundleBehavior');
  }
}
