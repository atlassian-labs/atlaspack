import type SourceMap from '@parcel/source-map';
import type {Readable} from 'stream';
import type {FileSystem} from '@atlaspack/fs';

import type {
  Asset as IAsset,
  AST,
  ASTGenerator,
  Dependency as IDependency,
  DependencyOptions,
  Environment as IEnvironment,
  EnvironmentOptions,
  FileCreateInvalidation,
  FilePath,
  Meta,
  MutableAsset as IMutableAsset,
  Stats,
  MutableAssetSymbols as IMutableAssetSymbols,
  AssetSymbols as IAssetSymbols,
  BundleBehavior,
} from '@atlaspack/types';
import type {Asset as AssetValue, AtlaspackOptions} from '../types';

import nullthrows from 'nullthrows';
import Environment from './Environment';
import {getPublicDependency} from './Dependency';
import {AssetSymbols, MutableAssetSymbols} from './Symbols';
import UncommittedAsset from '../UncommittedAsset';
import CommittedAsset from '../CommittedAsset';
import {createEnvironment} from '../Environment';
import {fromProjectPath, toProjectPath} from '../projectPath';
import {
  BundleBehavior as BundleBehaviorMap,
  BundleBehaviorNames,
} from '../types';
import {toInternalSourceLocation} from '../utils';

const inspect = Symbol.for('nodejs.util.inspect.custom');

const uncommittedAssetValueToAsset: WeakMap<AssetValue, Asset> = new WeakMap();
const committedAssetValueToAsset: WeakMap<AssetValue, Asset> = new WeakMap();
const assetValueToMutableAsset: WeakMap<AssetValue, MutableAsset> =
  new WeakMap();

const _assetToAssetValue: WeakMap<
  IAsset | IMutableAsset | BaseAsset,
  AssetValue
> = new WeakMap();

const _mutableAssetToUncommittedAsset: WeakMap<
  IMutableAsset,
  UncommittedAsset
> = new WeakMap();

export function assetToAssetValue(asset: IAsset | IMutableAsset): AssetValue {
  return nullthrows(_assetToAssetValue.get(asset));
}

export function mutableAssetToUncommittedAsset(
  mutableAsset: IMutableAsset,
): UncommittedAsset {
  return nullthrows(_mutableAssetToUncommittedAsset.get(mutableAsset));
}

export function assetFromValue(
  value: AssetValue,
  options: AtlaspackOptions,
): Asset {
  return new Asset(
    value.committed
      ? new CommittedAsset(value, options)
      : new UncommittedAsset({
          value,
          options,
        }),
  );
}

class BaseAsset {
  #asset: CommittedAsset | UncommittedAsset;
  // @ts-expect-error - TS7008 - Member '#query' implicitly has an 'any' type.
  #query /*: ?URLSearchParams */;

  constructor(asset: CommittedAsset | UncommittedAsset) {
    this.#asset = asset;
    _assetToAssetValue.set(this, asset.value);
  }

  // $FlowFixMe[unsupported-syntax]
  [inspect](): string {
    return `Asset(${this.filePath})`;
  }

  get id(): string {
    return this.#asset.value.id;
  }

  get type(): string {
    return this.#asset.value.type;
  }

  get env(): IEnvironment {
    return new Environment(this.#asset.value.env, this.#asset.options);
  }

  get fs(): FileSystem {
    return this.#asset.options.inputFS;
  }

  get filePath(): FilePath {
    return fromProjectPath(
      this.#asset.options.projectRoot,
      this.#asset.value.filePath,
    );
  }

  get query(): URLSearchParams {
    if (!this.#query) {
      this.#query = new URLSearchParams(this.#asset.value.query ?? '');
    }
    return this.#query;
  }

  get meta(): Meta {
    return this.#asset.value.meta;
  }

  get bundleBehavior(): BundleBehavior | null | undefined {
    let bundleBehavior = this.#asset.value.bundleBehavior;
    return bundleBehavior == null ? null : BundleBehaviorNames[bundleBehavior];
  }

  get isBundleSplittable(): boolean {
    return this.#asset.value.isBundleSplittable;
  }

  get isSource(): boolean {
    return this.#asset.value.isSource;
  }

  get sideEffects(): boolean {
    return this.#asset.value.sideEffects;
  }

  get symbols(): IAssetSymbols {
    // @ts-expect-error - TS2322 - Type 'import("/home/ubuntu/parcel/packages/core/core/src/public/Symbols").AssetSymbols' is not assignable to type 'import("/home/ubuntu/parcel/packages/core/types-internal/src/index").AssetSymbols'.
    return new AssetSymbols(this.#asset.options, this.#asset.value);
  }

  get uniqueKey(): string | null | undefined {
    return this.#asset.value.uniqueKey;
  }

  get astGenerator(): ASTGenerator | null | undefined {
    return this.#asset.value.astGenerator;
  }

  get pipeline(): string | null | undefined {
    return this.#asset.value.pipeline;
  }

  getDependencies(): ReadonlyArray<IDependency> {
    return this.#asset
      .getDependencies()
      .map((dep) => getPublicDependency(dep, this.#asset.options));
  }

  getCode(): Promise<string> {
    return this.#asset.getCode();
  }

  getBuffer(): Promise<Buffer> {
    return this.#asset.getBuffer();
  }

  getStream(): Readable {
    return this.#asset.getStream();
  }

  getMap(): Promise<SourceMap | null | undefined> {
    return this.#asset.getMap();
  }

  getAST(): Promise<AST | null | undefined> {
    return this.#asset.getAST();
  }

  getMapBuffer(): Promise<Buffer | null | undefined> {
    return this.#asset.getMapBuffer();
  }
}

export class Asset extends BaseAsset implements IAsset {
  #asset /*: CommittedAsset | UncommittedAsset */;
  // @ts-expect-error - TS7008 - Member '#env' implicitly has an 'any' type.
  #env /*: ?Environment */;

  // @ts-expect-error - TS2376 - A 'super' call must be the first statement in the constructor when a class contains initialized properties, parameter properties, or private identifiers.
  constructor(asset: CommittedAsset | UncommittedAsset) {
    let assetValueToAsset = asset.value.committed
      ? committedAssetValueToAsset
      : uncommittedAssetValueToAsset;
    let existing = assetValueToAsset.get(asset.value);
    if (existing != null) {
      return existing;
    }

    super(asset);
    this.#asset = asset;
    assetValueToAsset.set(asset.value, this);
    return this;
  }

  get env(): IEnvironment {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'. | TS2532 - Object is possibly 'undefined'.
    this.#env ??= new Environment(this.#asset.value.env, this.#asset.options);
    return this.#env;
  }

  get stats(): Stats {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#asset.value.stats;
  }
}

export class MutableAsset extends BaseAsset implements IMutableAsset {
  #asset /*: UncommittedAsset */;

  // @ts-expect-error - TS2376 - A 'super' call must be the first statement in the constructor when a class contains initialized properties, parameter properties, or private identifiers.
  constructor(asset: UncommittedAsset) {
    let existing = assetValueToMutableAsset.get(asset.value);
    if (existing != null) {
      return existing;
    }

    super(asset);
    this.#asset = asset;
    assetValueToMutableAsset.set(asset.value, this);
    _mutableAssetToUncommittedAsset.set(this, asset);
    return this;
  }

  setMap(map?: SourceMap | null): void {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    this.#asset.setMap(map);
  }

  get type(): string {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#asset.value.type;
  }

  // @ts-expect-error - TS1095 - A 'set' accessor cannot have a return type annotation.
  set type(type: string): void {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    if (type !== this.#asset.value.type) {
      // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
      this.#asset.value.type = type;
      // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
      this.#asset.updateId();
    }
  }

  // @ts-expect-error - TS2380 - The return type of a 'get' accessor must be assignable to its 'set' accessor type
  get bundleBehavior(): BundleBehavior | null | undefined {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    let bundleBehavior = this.#asset.value.bundleBehavior;
    return bundleBehavior == null ? null : BundleBehaviorNames[bundleBehavior];
  }

  // @ts-expect-error - TS1095 - A 'set' accessor cannot have a return type annotation.
  set bundleBehavior(bundleBehavior?: BundleBehavior | null): void {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    this.#asset.value.bundleBehavior = bundleBehavior
      ? BundleBehaviorMap[bundleBehavior]
      : null;
  }

  get isBundleSplittable(): boolean {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#asset.value.isBundleSplittable;
  }

  // @ts-expect-error - TS1095 - A 'set' accessor cannot have a return type annotation.
  set isBundleSplittable(isBundleSplittable: boolean): void {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    this.#asset.value.isBundleSplittable = isBundleSplittable;
  }

  get sideEffects(): boolean {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#asset.value.sideEffects;
  }

  // @ts-expect-error - TS1095 - A 'set' accessor cannot have a return type annotation.
  set sideEffects(sideEffects: boolean): void {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    this.#asset.value.sideEffects = sideEffects;
  }

  // @ts-expect-error - TS2380 - The return type of a 'get' accessor must be assignable to its 'set' accessor type
  get uniqueKey(): string | null | undefined {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#asset.value.uniqueKey;
  }

  // @ts-expect-error - TS1095 - A 'set' accessor cannot have a return type annotation.
  set uniqueKey(uniqueKey?: string | null): void {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    if (this.#asset.value.uniqueKey != null) {
      throw new Error(
        "Cannot change an asset's uniqueKey after it has been set.",
      );
    }
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    this.#asset.value.uniqueKey = uniqueKey;
  }

  get symbols(): IMutableAssetSymbols {
    // @ts-expect-error - TS2322 - Type 'import("/home/ubuntu/parcel/packages/core/core/src/public/Symbols").MutableAssetSymbols' is not assignable to type 'import("/home/ubuntu/parcel/packages/core/types-internal/src/index").MutableAssetSymbols'. | TS2532 - Object is possibly 'undefined'. | TS2532 - Object is possibly 'undefined'.
    return new MutableAssetSymbols(this.#asset.options, this.#asset.value);
  }

  addDependency(dep: DependencyOptions): string {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#asset.addDependency(dep);
  }

  invalidateOnFileChange(filePath: FilePath): void {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    this.#asset.invalidateOnFileChange(
      // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
      toProjectPath(this.#asset.options.projectRoot, filePath),
    );
  }

  invalidateOnFileCreate(invalidation: FileCreateInvalidation): void {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    this.#asset.invalidateOnFileCreate(invalidation);
  }

  invalidateOnEnvChange(env: string): void {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    this.#asset.invalidateOnEnvChange(env);
  }

  invalidateOnStartup(): void {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    this.#asset.invalidateOnStartup();
  }

  invalidateOnBuild(): void {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    this.#asset.invalidateOnBuild();
  }

  isASTDirty(): boolean {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#asset.isASTDirty;
  }

  setBuffer(buffer: Buffer): void {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    this.#asset.setBuffer(buffer);
  }

  setCode(code: string): void {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    this.#asset.setCode(code);
  }

  setStream(stream: Readable): void {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    this.#asset.setStream(stream);
  }

  setAST(ast: AST): void {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#asset.setAST(ast);
  }

  addURLDependency(url: string, opts: Partial<DependencyOptions>): string {
    return this.addDependency({
      specifier: url,
      specifierType: 'url',
      priority: 'lazy',
      ...opts,
    });
  }

  setEnvironment(env: EnvironmentOptions): void {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    this.#asset.value.env = createEnvironment({
      ...env,
      // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
      loc: toInternalSourceLocation(this.#asset.options.projectRoot, env.loc),
    });
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    this.#asset.updateId();
  }
}
