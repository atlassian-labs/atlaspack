// @flow strict-local

import type {
  AST,
  Blob,
  DependencyOptions,
  FileCreateInvalidation,
  GenerateOutput,
  PackageName,
  TransformerResult,
} from '@atlaspack/types';
import type {Asset, Dependency, AtlaspackOptions, Invalidations} from './types';

import invariant from 'assert';
import {Readable} from 'stream';
import SourceMap from '@parcel/source-map';
import {serializeRaw} from '@atlaspack/build-cache';
import {
  blobToStream,
  bufferStream,
  streamFromPromise,
  TapStream,
  loadSourceMap,
  SOURCEMAP_RE,
} from '@atlaspack/utils';
import {hashString, hashBuffer, Hash} from '@atlaspack/rust';
import {createDependency, mergeDependencies} from './Dependency';
import {mergeEnvironments} from './Environment';
import {ATLASPACK_VERSION} from './constants';
import {createAsset, createAssetIdFromOptions} from './assetUtils';
import {BundleBehaviorNames} from './types';
import {invalidateOnFileCreateToInternal, createInvalidations} from './utils';
import {
  type ProjectPath,
  fromProjectPath,
  fromProjectPathRelative,
} from './projectPath';
import {getFeatureFlag} from '@atlaspack/feature-flags';
import {fromEnvironmentId} from './EnvironmentManager';

type UncommittedAssetOptions = {|
  value: Asset,
  options: AtlaspackOptions,
  content?: ?Blob,
  mapBuffer?: ?Buffer,
  ast?: ?AST,
  isASTDirty?: ?boolean,
  code?: ?string,
  invalidations?: Invalidations,
|};

export default class UncommittedAsset {
  value: Asset;
  options: AtlaspackOptions;
  content: ?(Blob | Promise<Buffer>);
  mapBuffer: ?Buffer;
  sourceContent: ?string;
  map: ?SourceMap;
  ast: ?AST;
  isASTDirty: boolean;
  code: ?string;
  invalidations: Invalidations;
  generate: ?() => Promise<GenerateOutput>;

  constructor({
    value,
    options,
    content,
    mapBuffer,
    ast,
    isASTDirty,
    code,
    invalidations,
  }: UncommittedAssetOptions) {
    this.value = value;
    this.options = options;
    this.content = content;
    this.mapBuffer = mapBuffer;
    this.ast = ast;
    this.isASTDirty = isASTDirty || false;
    this.code = code;
    this.invalidations = invalidations || createInvalidations();
  }

  /*
   * Prepares the asset for being serialized to the cache by committing its
   * content and map of the asset to the cache.
   */
  async commit(): Promise<void> {
    // If there is a dirty AST, clear out any old content and map as these
    // must be regenerated later and shouldn't be committed.
    if (this.ast != null && this.isASTDirty) {
      this.content = null;
      this.mapBuffer = null;
    }

    let size = 0;
    let outputHash = '';
    let contentKey = this.content == null ? null : this.getCacheKey('content');
    let mapKey = this.mapBuffer == null ? null : this.getCacheKey('map');
    let astKey = this.ast == null ? null : this.getCacheKey('ast');

    // Since we can only read from the stream once, compute the content length
    // and hash while it's being written to the cache.
    await Promise.all([
      contentKey != null &&
        this.commitContent(contentKey).then(
          (s) => ((size = s.size), (outputHash = s.hash)),
        ),
      this.mapBuffer != null &&
        mapKey != null &&
        this.options.cache.setBlob(mapKey, this.mapBuffer),
      astKey != null &&
        this.options.cache.setBlob(astKey, serializeRaw(this.ast)),
    ]);
    this.value.contentKey = contentKey;
    this.value.mapKey = mapKey;
    this.value.astKey = astKey;
    this.value.outputHash = outputHash;

    if (this.content != null) {
      this.value.stats.size = size;
    }

    this.value.isLargeBlob = this.content instanceof Readable;
    this.value.committed = true;
  }

  async commitContent(
    contentKey: string,
  ): Promise<{|size: number, hash: string|}> {
    let content = await this.content;
    if (content == null) {
      return {size: 0, hash: ''};
    }

    let size = 0;
    if (content instanceof Readable) {
      let hash = new Hash();
      await this.options.cache.setStream(
        contentKey,
        content.pipe(
          new TapStream((buf) => {
            hash.writeBuffer(buf);
            size += buf.length;
          }),
        ),
      );

      return {size, hash: hash.finish()};
    }

    let hash;
    if (typeof content === 'string') {
      hash = hashString(content);
      size = Buffer.byteLength(content);
    } else {
      hash = hashBuffer(content);
      size = content.length;
    }

    // Maybe we should just store this in a file instead of LMDB
    await this.options.cache.setBlob(contentKey, content);

    return {size, hash};
  }

  async getCode(): Promise<string> {
    if (this.ast != null && this.isASTDirty) {
      throw new Error(
        'Cannot call getCode() on an asset with a dirty AST. For transformers, implement canReuseAST() and check asset.isASTDirty.',
      );
    }

    let content = await this.content;
    if (typeof content === 'string' || content instanceof Buffer) {
      return content.toString();
    } else if (content != null) {
      this.content = bufferStream(content);
      return (await this.content).toString();
    }

    invariant(false, 'Internal error: missing content');
  }

  async getBuffer(): Promise<Buffer> {
    let content = await this.content;
    if (content == null) {
      return Buffer.alloc(0);
    } else if (content instanceof Buffer) {
      return content;
    } else if (typeof content === 'string') {
      return Buffer.from(content);
    }

    this.content = bufferStream(content);
    return this.content;
  }

  getStream(): Readable {
    if (this.content instanceof Readable) {
      // Remove content if it's a stream, as it should not be reused.
      let content = this.content;
      this.content = null;
      return content;
    }

    if (this.content instanceof Promise) {
      return streamFromPromise(this.content);
    }

    return blobToStream(this.content ?? Buffer.alloc(0));
  }

  setCode(code: string) {
    this.content = code;
    this.clearAST();
  }

  setBuffer(buffer: Buffer) {
    this.content = buffer;
    this.clearAST();
  }

  /**
   * @deprecated This has been broken on any cache other than FSCache for a long time.
   */
  setStream(stream: Readable) {
    this.content = stream;
    this.clearAST();
  }

  async loadExistingSourcemap(): Promise<?SourceMap> {
    if (this.map) {
      return this.map;
    }

    let code = await this.getCode();
    let map = await loadSourceMap(
      fromProjectPath(this.options.projectRoot, this.value.filePath),
      code,
      {
        fs: this.options.inputFS,
        projectRoot: this.options.projectRoot,
      },
    );

    if (map) {
      this.map = map;
      this.mapBuffer = map.toBuffer();
      this.setCode(code.replace(SOURCEMAP_RE, ''));
    }

    return this.map;
  }

  getMapBuffer(): Promise<?Buffer> {
    return Promise.resolve(this.mapBuffer);
  }

  async getMap(): Promise<?SourceMap> {
    if (this.map == null) {
      let mapBuffer = this.mapBuffer ?? (await this.getMapBuffer());
      if (mapBuffer) {
        // Get sourcemap from flatbuffer
        this.map = new SourceMap(this.options.projectRoot, mapBuffer);
      }
    }

    return this.map;
  }

  setMap(map: ?SourceMap): void {
    // If we have sourceContent available, it means this asset is source code without
    // a previous source map. Ensure that the map set by the transformer has the original
    // source content available.
    if (map != null && this.sourceContent != null) {
      map.setSourceContent(
        fromProjectPath(this.options.projectRoot, this.value.filePath),
        // $FlowFixMe
        this.sourceContent,
      );
      this.sourceContent = null;
    }

    this.map = map;
    this.mapBuffer = this.map?.toBuffer();
  }

  getAST(): Promise<?AST> {
    return Promise.resolve(this.ast);
  }

  setAST(ast: AST): void {
    this.ast = ast;
    this.isASTDirty = true;
    this.value.astGenerator = {
      type: ast.type,
      version: ast.version,
    };
  }

  clearAST() {
    this.ast = null;
    this.isASTDirty = false;
    this.value.astGenerator = null;
  }

  getCacheKey(key: string): string {
    if (getFeatureFlag('cachePerformanceImprovements')) {
      const filePath = fromProjectPathRelative(this.value.filePath);
      return `Asset/${ATLASPACK_VERSION}/${filePath}/${this.value.id}/${key}`;
    }

    return hashString(ATLASPACK_VERSION + key + this.value.id);
  }

  addDependency(opts: DependencyOptions): string {
    // eslint-disable-next-line no-unused-vars
    let {env, symbols, ...rest} = opts;
    let dep = createDependency(this.options.projectRoot, {
      ...rest,
      // $FlowFixMe "convert" the $ReadOnlyMaps to the interal mutable one
      symbols,
      env: mergeEnvironments(
        this.options.projectRoot,
        fromEnvironmentId(this.value.env),
        env,
      ),
      sourceAssetId: this.value.id,
      sourcePath: fromProjectPath(
        this.options.projectRoot,
        this.value.filePath,
      ),
    });
    let existing = this.value.dependencies.get(dep.id);
    if (existing) {
      mergeDependencies(existing, dep);
    } else {
      this.value.dependencies.set(dep.id, dep);
    }
    return dep.id;
  }

  invalidateOnFileChange(filePath: ProjectPath) {
    this.invalidations.invalidateOnFileChange.add(filePath);
  }

  invalidateOnFileCreate(invalidation: FileCreateInvalidation) {
    this.invalidations.invalidateOnFileCreate.push(
      invalidateOnFileCreateToInternal(this.options.projectRoot, invalidation),
    );
  }

  invalidateOnEnvChange(key: string) {
    this.invalidations.invalidateOnEnvChange.add(key);
  }

  invalidateOnBuild() {
    this.invalidations.invalidateOnBuild = true;
  }

  invalidateOnStartup() {
    this.invalidations.invalidateOnStartup = true;
  }

  getDependencies(): Array<Dependency> {
    return Array.from(this.value.dependencies.values());
  }

  createChildAsset(
    result: TransformerResult,
    plugin: PackageName,
    configPath: ProjectPath,
    configKeyPath?: string,
  ): UncommittedAsset {
    let content = result.content ?? null;

    let asset = new UncommittedAsset({
      value: createAsset(this.options.projectRoot, {
        code: this.code,
        filePath: this.value.filePath,
        type: result.type,
        bundleBehavior:
          result.bundleBehavior ??
          (this.value.bundleBehavior == null
            ? null
            : BundleBehaviorNames[this.value.bundleBehavior]),
        isBundleSplittable:
          result.isBundleSplittable ?? this.value.isBundleSplittable,
        isSource: this.value.isSource,
        env: mergeEnvironments(
          this.options.projectRoot,
          fromEnvironmentId(this.value.env),
          result.env,
        ),
        dependencies:
          this.value.type === result.type
            ? new Map(this.value.dependencies)
            : new Map(),
        meta: {
          ...this.value.meta,
          ...result.meta,
        },
        pipeline:
          result.pipeline ??
          (this.value.type === result.type ? this.value.pipeline : null),
        stats: {
          time: 0,
          size: this.value.stats.size,
        },
        // $FlowFixMe
        symbols: result.symbols,
        sideEffects: result.sideEffects ?? this.value.sideEffects,
        uniqueKey: result.uniqueKey,
        astGenerator: result.ast
          ? {type: result.ast.type, version: result.ast.version}
          : null,
        plugin,
        configPath,
        configKeyPath,
      }),
      options: this.options,
      content,
      ast: result.ast,
      isASTDirty: result.ast === this.ast ? this.isASTDirty : true,
      mapBuffer: result.map ? result.map.toBuffer() : null,
      code: this.code,
      invalidations: this.invalidations,
    });

    let dependencies = result.dependencies;
    if (dependencies) {
      for (let dep of dependencies) {
        asset.addDependency(dep);
      }
    }

    return asset;
  }

  updateId() {
    // $FlowFixMe - this is fine
    this.value.id = createAssetIdFromOptions(this.value);
  }
}
