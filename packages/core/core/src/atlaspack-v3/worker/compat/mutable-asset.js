// @flow

import SourceMap from '@parcel/source-map';
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
  bundleBehavior: ?BundleBehavior;
  env: Environment;
  filePath: FilePath;
  fs: FileSystem;
  id: string;
  isBundleSplittable: boolean;
  isMapDirty: boolean;
  isSource: boolean;
  meta: Meta;
  pipeline: ?string;
  query: URLSearchParams;
  sideEffects: boolean;
  stats: Stats;
  symbols: MutableAssetSymbols;
  type: string;
  uniqueKey: ?string;

  #astDirty: boolean;
  #ast: ?AST;
  #contents: Buffer;
  #inner: InnerAsset;
  #map: ?string;
  #projectRoot: string;
  #sourceMap: ?SourceMap;

  get astGenerator(): ?ASTGenerator {
    throw new Error('get MutableAsset.astGenerator');
  }

  set astGenerator(value: ?ASTGenerator) {
    throw new Error('set MutableAsset.astGenerator');
  }

  constructor(
    asset: InnerAsset,
    contents: Buffer,
    env: Environment,
    fs: FileSystem,
    map: ?string,
    projectRoot: string,
  ) {
    this.bundleBehavior = bundleBehaviorMap.fromNullable(asset.bundleBehavior);
    this.env = env;
    this.filePath = asset.filePath;
    this.fs = fs;
    this.id = asset.id;
    this.isBundleSplittable = asset.isBundleSplittable;
    this.isSource = asset.isSource;
    this.meta = asset.meta;
    this.pipeline = asset.pipeline;
    this.query = new URLSearchParams(asset.query);
    this.sideEffects = asset.sideEffects;
    this.stats = asset.stats;
    this.symbols = new MutableAssetSymbols(asset.symbols);
    this.type = asset.type;
    this.uniqueKey = asset.uniqueKey;

    this.#astDirty = false;
    this.#contents = contents;
    this.#inner = asset;
    this.#map = map;
    this.#projectRoot = projectRoot;
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
    // Only create the source map if it is requested
    if (!this.#sourceMap && this.#map && typeof this.#map === 'string') {
      let sourceMap = new SourceMap(this.#projectRoot);
      // $FlowFixMe Flow is dumb
      sourceMap.addVLQMap(JSON.parse(this.#map));
      this.#sourceMap = sourceMap;
    }

    return Promise.resolve(this.#sourceMap);
  }

  // eslint-disable-next-line no-unused-vars
  setMap(sourceMap: ?SourceMap): void {
    this.isMapDirty = true;
    this.#sourceMap = sourceMap;
  }

  getMapBuffer(): Promise<?Buffer> {
    throw new Error(
      'getMapBuffer() is considered an internal implementation detail, please use getMap() instead',
    );
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
