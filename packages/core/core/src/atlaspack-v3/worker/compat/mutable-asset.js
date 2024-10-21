// @flow

import type SourceMap from '@parcel/source-map';
import * as napi from '@atlaspack/rust';
import {Readable} from 'stream';
import type {
  MutableAsset as IMutableAsset,
  Stats,
  FileSystem,
  FilePath,
  Environment,
  Meta,
  BundleBehavior,
  ASTGenerator,
  AST,
  Dependency,
  DependencyOptions,
  FileCreateInvalidation,
  EnvironmentOptions,
} from '@atlaspack/types';
import {bundleBehaviorMap} from './bitflags';
import {MutableAssetSymbols} from './asset-symbols';

export type InnerAsset = napi.Asset;

export class MutableAsset implements IMutableAsset {
  fs: FileSystem;
  env: Environment;
  symbols: MutableAssetSymbols;
  stats: Stats;
  id: string;
  filePath: FilePath;
  type: string;
  query: URLSearchParams;
  isSource: boolean;
  meta: Meta;
  bundleBehavior: ?BundleBehavior;
  isBundleSplittable: boolean;
  sideEffects: boolean;
  uniqueKey: ?string;
  pipeline: ?string;

  #inner: InnerAsset;
  #ast: ?AST;
  #contents: Buffer;
  #astDirty: boolean;

  get astGenerator(): ?ASTGenerator {
    throw new Error('get MutableAsset.astGenerator');
  }

  set astGenerator(value: ?ASTGenerator) {
    throw new Error('set MutableAsset.astGenerator');
  }

  constructor(asset: InnerAsset, fs: FileSystem, env: Environment) {
    this.#inner = asset;
    this.stats = asset.stats;
    this.id = asset.id;
    this.filePath = asset.filePath;
    this.type;
    this.symbols = new MutableAssetSymbols(asset.symbols);
    this.stats = asset.stats;
    this.id = asset.id;
    this.filePath = asset.filePath;
    this.type = asset.type;
    this.query = new URLSearchParams(asset.query);
    this.isSource = asset.isSource;
    this.meta = asset.meta;
    this.bundleBehavior = bundleBehaviorMap.fromNullable(asset.bundleBehavior);
    this.isBundleSplittable = asset.isBundleSplittable;
    this.sideEffects = asset.sideEffects;
    this.uniqueKey = asset.uniqueKey;
    this.pipeline = asset.pipeline;
    this.#contents = Buffer.from(asset.code);
    this.fs = fs;
    this.env = env;
    this.#astDirty = false;
  }

  // eslint-disable-next-line require-await
  async getAST(): Promise<?AST> {
    return this.#ast;
  }

  setAST(ast: AST): void {
    this.#astDirty = true;
    this.#ast = ast;
  }

  isASTDirty(): boolean {
    return this.#astDirty;
  }

  // eslint-disable-next-line require-await
  async getCode(): Promise<string> {
    return this.#contents.toString();
  }

  setCode(code: string): void {
    this.#contents = Buffer.from(code);
  }

  // eslint-disable-next-line require-await
  async getBuffer(): Promise<Buffer> {
    return this.#contents;
  }

  setBuffer(buf: Buffer): void {
    this.#contents = buf;
  }

  getStream(): Readable {
    return Readable.from(this.#contents);
  }

  setStream(stream: Readable): void {
    const data = [];

    stream.on('data', (chunk) => {
      data.push(chunk);
    });

    stream.on('end', () => {
      this.#contents = Buffer.concat(data);
    });

    stream.on('error', () => {
      throw new Error('MutableAsset.setStream()');
    });
  }

  getMap(): Promise<?SourceMap> {
    // TODO: Provide source maps once they exist
    return Promise.resolve(null);
  }

  // eslint-disable-next-line no-unused-vars
  setMap(sourceMap: ?SourceMap): void {
    throw new Error('MutableAsset.setMap()');
  }

  getMapBuffer(): Promise<?Buffer> {
    throw new Error('MutableAsset.getMapBuffer');
  }

  getDependencies(): $ReadOnlyArray<Dependency> {
    throw new Error('MutableAsset.getDependencies');
  }

  // eslint-disable-next-line no-unused-vars
  addDependency(options: DependencyOptions): string {
    throw new Error('MutableAsset.addDependency()');
  }

  // eslint-disable-next-line no-unused-vars
  addURLDependency(url: string, opts: $Shape<DependencyOptions>): string {
    throw new Error('MutableAsset.addURLDependency()');
  }

  // eslint-disable-next-line no-unused-vars
  setEnvironment(opts: EnvironmentOptions): void {
    throw new Error('MutableAsset.setEnvironment()');
  }

  // eslint-disable-next-line no-unused-vars
  invalidateOnFileChange(invalidation: FilePath): void {
    // TODO: Forward invalidations to Rust
  }

  // eslint-disable-next-line no-unused-vars
  invalidateOnFileCreate(invalidation: FileCreateInvalidation): void {
    // TODO: Forward invalidations to Rust
  }

  // eslint-disable-next-line no-unused-vars
  invalidateOnEnvChange(invalidation: string): void {
    // TODO: Forward invalidations to Rust
  }

  invalidateOnStartup(): void {
    // TODO: Forward invalidations to Rust
  }

  invalidateOnBuild(): void {
    // TODO: Forward invalidations to Rust
  }
}
