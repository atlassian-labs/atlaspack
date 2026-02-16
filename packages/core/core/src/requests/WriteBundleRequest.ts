import type {FileSystem, FileOptions} from '@atlaspack/fs';
import type {ContentKey} from '@atlaspack/graph';
import type {Async, FilePath, Compressor} from '@atlaspack/types';

import type {RunAPI, StaticRunOpts} from '../RequestTracker';
import type {Bundle, PackagedBundleInfo, AtlaspackOptions} from '../types';
import type BundleGraph from '../BundleGraph';
import type {BundleInfo} from '../PackagerRunner';
import type {ConfigAndCachePath} from './AtlaspackConfigRequest';
import type {LoadedPlugin} from '../AtlaspackConfig';
import type {ProjectPath} from '../projectPath';

import {HASH_REF_HASH_LEN, HASH_REF_PREFIX} from '../constants';
import nullthrows from 'nullthrows';
import path from 'path';
import url from 'url';
import {NamedBundle} from '../public/Bundle';
import {blobToStream, TapStream} from '@atlaspack/utils';
import {Readable, Transform, pipeline} from 'stream';
import {
  fromProjectPath,
  fromProjectPathRelative,
  toProjectPath,
  joinProjectPath,
  toProjectPathUnsafe,
} from '../projectPath';
import createAtlaspackConfigRequest, {
  getCachedAtlaspackConfig,
} from './AtlaspackConfigRequest';
import PluginOptions from '../public/PluginOptions';
import {PluginLogger} from '@atlaspack/logger';
import {
  getDevDepRequests,
  invalidateDevDeps,
  createDevDependency,
  runDevDepRequest,
} from './DevDepRequest';
import {AtlaspackConfig} from '../AtlaspackConfig';
import ThrowableDiagnostic, {errorToDiagnostic} from '@atlaspack/diagnostic';
import {PluginTracer, tracer} from '@atlaspack/profiler';
import {requestTypes} from '../RequestTracker';
import {getFeatureFlag} from '@atlaspack/feature-flags';
import {fromEnvironmentId} from '../EnvironmentManager';
import SourceMap, {decodeVLQ, encodeVLQ} from '@atlaspack/source-map';

const HASH_REF_PREFIX_LEN = HASH_REF_PREFIX.length;
const BOUNDARY_LENGTH = HASH_REF_PREFIX.length + 32 - 1;
const HASH_REF_PLACEHOLDER_LEN = HASH_REF_PREFIX_LEN + HASH_REF_HASH_LEN;

// The JSON key prefix we scan for in the source map stream.
const MAPPINGS_KEY_BUF = Buffer.from('"mappings":"');

export type HashRefReplacement = {
  line: number;
  column: number;
  originalLength: number;
  newLength: number;
};

type WriteBundleRequestInput = {
  bundleGraph: BundleGraph;
  bundle: Bundle;
  info: BundleInfo;
  hashRefToNameHash: Map<string, string>;
};

export type WriteBundleRequestResult = PackagedBundleInfo;

type RunInput<TResult> = {
  input: WriteBundleRequestInput;
} & StaticRunOpts<TResult>;

export type WriteBundleRequest = {
  id: ContentKey;
  readonly type: typeof requestTypes.write_bundle_request;
  run: (arg1: RunInput<PackagedBundleInfo>) => Async<PackagedBundleInfo>;
  input: WriteBundleRequestInput;
};

/**
 * Writes a bundle to the dist directory, replacing hash references with the final content hashes.
 */
export default function createWriteBundleRequest(
  input: WriteBundleRequestInput,
): WriteBundleRequest {
  let name = nullthrows(input.bundle.name);
  let nameHash = nullthrows(
    input.hashRefToNameHash.get(input.bundle.hashReference),
  );
  return {
    id: `${input.bundle.id}:${input.info.hash}:${nameHash}:${name}`,
    type: requestTypes.write_bundle_request,
    run,
    input,
  };
}

// @ts-expect-error TS7031
async function run({input, options, api}) {
  let {bundleGraph, bundle, info, hashRefToNameHash} = input;
  let {inputFS, outputFS} = options;
  let name = nullthrows(bundle.name);
  let thisHashReference = bundle.hashReference;

  if (info.type !== bundle.type) {
    name = name.slice(0, -path.extname(name).length) + '.' + info.type;
  }

  if (name.includes(thisHashReference)) {
    let thisNameHash = nullthrows(hashRefToNameHash.get(thisHashReference));
    name = name.replace(thisHashReference, thisNameHash);
  }

  let filePath = joinProjectPath(bundle.target.distDir, name);

  // Watch the bundle and source map for deletion.
  // Also watch the dist dir because invalidateOnFileDelete does not currently
  // invalidate when a parent directory is deleted.
  // TODO: do we want to also watch for file edits?
  api.invalidateOnFileDelete(bundle.target.distDir);
  api.invalidateOnFileDelete(filePath);

  let cacheKeys = info.cacheKeys;
  let mapKey = cacheKeys.map;
  let fullPath = fromProjectPath(options.projectRoot, filePath);
  const env = fromEnvironmentId(bundle.env);

  if (mapKey && env.sourceMap && !env.sourceMap.inline) {
    api.invalidateOnFileDelete(
      toProjectPath(options.projectRoot, fullPath + '.map'),
    );
  }

  let dir = path.dirname(fullPath);
  await outputFS.mkdirp(dir); // ? Got rid of dist exists, is this an expensive operation

  // Use the file mode from the entry asset as the file mode for the bundle.
  // Don't do this for browser builds, as the executable bit in particular is unnecessary.
  let publicBundle = NamedBundle.get(bundle, bundleGraph, options);
  let mainEntry = publicBundle.getMainEntry();
  let writeOptions =
    publicBundle.env.isBrowser() || !mainEntry
      ? undefined
      : {
          mode: (await inputFS.stat(mainEntry.filePath)).mode,
        };
  let contentStream: Readable;
  if (info.isLargeBlob) {
    contentStream = options.cache.getStream(cacheKeys.content);
  } else {
    contentStream = blobToStream(
      await options.cache.getBlob(cacheKeys.content),
    );
  }
  let size = 0;
  contentStream = contentStream.pipe(
    // @ts-expect-error TS2554
    new TapStream((buf: Buffer) => {
      size += buf.length;
    }),
  );

  let configResult = nullthrows(
    // @ts-expect-error TS2347
    await api.runRequest<null, ConfigAndCachePath>(
      createAtlaspackConfigRequest(),
    ),
  );
  let config = getCachedAtlaspackConfig(configResult, options);

  let {devDeps, invalidDevDeps} = await getDevDepRequests(api);
  invalidateDevDeps(invalidDevDeps, options, config);

  const bundleReplacements = getFeatureFlag('fixSourceMapHashRefs')
    ? []
    : undefined;
  await writeFiles(
    contentStream,
    info,
    hashRefToNameHash,
    options,
    config,
    outputFS,
    filePath,
    writeOptions,
    devDeps,
    api,
    bundleReplacements,
  );

  const hasSourceMap = await options.cache.has(mapKey);
  if (mapKey && env.sourceMap && !env.sourceMap.inline && hasSourceMap) {
    const mapEntry = await options.cache.getBlob(mapKey);
    let mapStream: Readable;
    if (
      getFeatureFlag('fixSourceMapHashRefs') &&
      bundleReplacements &&
      bundleReplacements.length > 0
    ) {
      mapStream = blobToStream(mapEntry).pipe(
        new SourceMapHashRefRewriteStream(bundleReplacements),
      );
    } else {
      mapStream = blobToStream(mapEntry);
    }
    await writeFiles(
      mapStream,
      info,
      hashRefToNameHash,
      options,
      config,
      outputFS,
      toProjectPathUnsafe(fromProjectPathRelative(filePath) + '.map'),
      writeOptions,
      devDeps,
      api,
    );
  }

  let res = {
    filePath,
    bundleId: bundle.id,
    type: info.type,
    stats: {
      size,
      time: info.time ?? 0,
    },
  };

  api.storeResult(res);
  return res;
}

export function applyReplacementsToSourceMap(
  sourceMap: SourceMap,
  replacements: HashRefReplacement[],
): void {
  if (replacements.length === 0) return;
  const sorted = [...replacements].sort(
    (a, b) => a.line - b.line || a.column - b.column,
  );
  for (const r of sorted) {
    const delta = r.newLength - r.originalLength;
    if (delta !== 0) {
      // r.column is in post-replacement coordinates (matching the already-shifted
      // source map state after previous offsetColumns calls). The end of the
      // placeholder in these coordinates is simply r.column + r.originalLength.
      const offsetStartColumn = r.column + r.originalLength;
      const line1Based = r.line + 1;
      if (line1Based >= 1 && offsetStartColumn + delta >= 0) {
        sourceMap.offsetColumns(line1Based, offsetStartColumn, delta);
      }
    }
  }
}

/**
 * Applies hash-ref replacement column offsets directly to a VLQ mappings
 * string without deserializing the full source map into a native struct.
 *
 * Each replacement r describes a hash-ref that was substituted in the output
 * file.  r.column is in the progressively-shifted post-replacement coordinate
 * space (matching the already-shifted source map state after all previous
 * offsetColumns calls), so thresholds are applied sequentially against the
 * running absCol values exactly as the native offsetColumns implementation does.
 */
export function applyReplacementsToVLQMappings(
  mappings: string,
  replacements: HashRefReplacement[],
): string {
  if (replacements.length === 0) return mappings;

  // Group replacements by line (0-indexed), sorted by column ascending.
  const byLine = new Map<number, HashRefReplacement[]>();
  for (const r of replacements) {
    let arr = byLine.get(r.line);
    if (!arr) {
      arr = [];
      byLine.set(r.line, arr);
    }
    arr.push(r);
  }
  for (const arr of byLine.values()) {
    arr.sort((a, b) => a.column - b.column);
  }

  const lines = mappings.split(';');
  const resultLines: string[] = [];

  for (let lineIdx = 0; lineIdx < lines.length; lineIdx++) {
    const lineReps = byLine.get(lineIdx);
    if (!lineReps || lineReps.length === 0) {
      resultLines.push(lines[lineIdx]);
      continue;
    }

    const line = lines[lineIdx];
    if (!line) {
      resultLines.push('');
      continue;
    }

    // Decode segment column deltas to absolute columns.
    const segments = line.split(',');
    const colVlqEnds: number[] = [];
    const absCols: number[] = [];
    let absCol = 0;
    for (const seg of segments) {
      const {value: colDelta, nextPos} = decodeVLQ(seg, 0);
      absCol += colDelta;
      colVlqEnds.push(nextPos);
      absCols.push(absCol);
    }

    // Apply each replacement's column shift sequentially against the
    // current absCol values (which have already been adjusted by previous
    // replacements on this line), mirroring the sequential offsetColumns calls.
    for (const r of lineReps) {
      const delta = r.newLength - r.originalLength;
      if (delta === 0) continue;
      const threshold = r.column + r.originalLength;
      for (let i = 0; i < absCols.length; i++) {
        if (absCols[i] >= threshold) {
          absCols[i] += delta;
        }
      }
    }

    // Re-encode with updated absolute columns; only the leading column VLQ
    // field of each segment changes – the tail bytes are sliced unchanged.
    const resultSegments: string[] = [];
    let prevAbsCol = 0;
    for (let i = 0; i < segments.length; i++) {
      const newDelta = absCols[i] - prevAbsCol;
      prevAbsCol = absCols[i];
      resultSegments.push(
        encodeVLQ(newDelta) + segments[i].slice(colVlqEnds[i]),
      );
    }

    resultLines.push(resultSegments.join(','));
  }

  return resultLines.join(';');
}

type StreamState = 'scanning' | 'buffering' | 'passthrough';

/**
 * A Transform stream that rewrites the "mappings" VLQ field of a source map
 * JSON to account for hash-ref replacements, without ever loading the full
 * JSON object or the native Rust SourceMapInner into memory.
 *
 * Field order in cached source maps (from partialVlqMapToSourceMap / toVLQ):
 *   mappings → sources → sourcesContent → names → version → file → sourceRoot
 *
 * "mappings" is the very first field, so we scan only a tiny header before
 * switching to zero-copy passthrough for the bulk sourcesContent bytes.
 */
export class SourceMapHashRefRewriteStream extends Transform {
  private replacements: HashRefReplacement[];
  private state: StreamState;
  private scanBuf: Buffer;
  private mappingsBufs: Buffer[];

  constructor(replacements: HashRefReplacement[]) {
    super();
    this.replacements = replacements;
    this.state = 'scanning';
    this.scanBuf = Buffer.alloc(0);
    this.mappingsBufs = [];
  }

  // @ts-expect-error TS7006
  _transform(chunk: Buffer, _encoding: string, cb): void {
    if (this.state === 'passthrough') {
      this.push(chunk);
      cb();
      return;
    }

    if (this.state === 'scanning') {
      const combined = Buffer.concat([this.scanBuf, chunk]);
      const idx = combined.indexOf(MAPPINGS_KEY_BUF);

      if (idx === -1) {
        // Key not yet found – hold back enough bytes to handle a split key.
        const keepLen = Math.min(combined.length, MAPPINGS_KEY_BUF.length - 1);
        if (combined.length > keepLen) {
          this.push(combined.slice(0, combined.length - keepLen));
        }
        this.scanBuf = combined.slice(combined.length - keepLen);
        cb();
        return;
      }

      // Emit everything up to and including the key.
      const keyEnd = idx + MAPPINGS_KEY_BUF.length;
      this.push(combined.slice(0, keyEnd));
      this.scanBuf = Buffer.alloc(0);
      this.state = 'buffering';
      this._bufferingTransform(combined.slice(keyEnd), cb);
      return;
    }

    // state === 'buffering'
    this._bufferingTransform(chunk, cb);
  }

  // @ts-expect-error TS7006
  private _bufferingTransform(chunk: Buffer, cb): void {
    // Mappings values contain only base64 chars, ';', and ',' – no escaping –
    // so scanning for the closing '"' (0x22) is safe.
    const closeIdx = chunk.indexOf(0x22);

    if (closeIdx === -1) {
      this.mappingsBufs.push(chunk);
      cb();
      return;
    }

    this.mappingsBufs.push(chunk.slice(0, closeIdx));

    // VLQ chars are all ASCII (<128), so latin1 round-trips without loss.
    const mappingsStr = Buffer.concat(this.mappingsBufs).toString('latin1');
    const rewritten = applyReplacementsToVLQMappings(
      mappingsStr,
      this.replacements,
    );
    this.push(Buffer.from(rewritten, 'latin1'));

    // Emit the closing '"' and everything remaining in one push.
    this.push(chunk.slice(closeIdx));

    this.state = 'passthrough';
    this.mappingsBufs = [];
    cb();
  }

  // @ts-expect-error TS7006
  _flush(cb): void {
    if (this.state === 'scanning' && this.scanBuf.length > 0) {
      this.push(this.scanBuf);
    } else if (this.state === 'buffering') {
      // Malformed JSON – flush whatever we buffered as-is.
      this.push(Buffer.concat(this.mappingsBufs));
    }
    cb();
  }
}

/**
 * Computes the sourceRoot for a source map file. This is the relative path from
 * the output directory back to the project root, so that source paths (stored
 * relative to projectRoot) resolve correctly from the .map file location.
 *
 * Returns undefined when sources are inlined (inlineSources), since the browser
 * doesn't need to fetch them and sourceRoot would be unnecessary.
 *
 * This logic must stay in sync with PackagerRunner.generateSourceMap.
 */
export function computeSourceMapRoot(
  bundle: Bundle,
  options: AtlaspackOptions,
): string | undefined {
  let name = nullthrows(bundle.name);
  let filePath = joinProjectPath(bundle.target.distDir, name);
  let fullPath = fromProjectPath(options.projectRoot, filePath);
  let sourceRoot: string = path.relative(
    path.dirname(fullPath),
    options.projectRoot,
  );

  let inlineSources = false;

  const bundleEnv = fromEnvironmentId(bundle.env);
  if (bundle.target) {
    const bundleTargetEnv = fromEnvironmentId(bundle.target.env);

    if (bundleEnv.sourceMap && bundleEnv.sourceMap.sourceRoot !== undefined) {
      sourceRoot = bundleEnv.sourceMap.sourceRoot;
    } else if (options.serveOptions && bundleTargetEnv.context === 'browser') {
      sourceRoot = '/__parcel_source_root';
    }

    if (
      bundleEnv.sourceMap &&
      bundleEnv.sourceMap.inlineSources !== undefined
    ) {
      inlineSources = bundleEnv.sourceMap.inlineSources;
    } else if (bundleTargetEnv.context !== 'node') {
      inlineSources = options.mode === 'production';
    }
  }

  let isInlineMap = bundleEnv.sourceMap && bundleEnv.sourceMap.inline;

  if (getFeatureFlag('omitSourcesContentInMemory') && !isInlineMap) {
    if (!(bundleEnv.sourceMap && bundleEnv.sourceMap.inlineSources === false)) {
      inlineSources = true;
    }
  }

  if (inlineSources) {
    return undefined;
  }

  return url.format(url.parse(sourceRoot + '/'));
}

async function writeFiles(
  // @ts-expect-error TS2503
  inputStream: stream.Readable,
  info: BundleInfo,
  hashRefToNameHash: Map<string, string>,
  options: AtlaspackOptions,
  config: AtlaspackConfig,
  outputFS: FileSystem,
  filePath: ProjectPath,
  writeOptions: FileOptions | null | undefined,
  devDeps: Map<string, string>,
  api: RunAPI<PackagedBundleInfo>,
  bundleReplacements?: HashRefReplacement[],
) {
  let compressors = await config.getCompressors(
    fromProjectPathRelative(filePath),
  );
  let fullPath = fromProjectPath(options.projectRoot, filePath);

  let stream = info.hashReferences.length
    ? inputStream.pipe(replaceStream(hashRefToNameHash, bundleReplacements))
    : inputStream;

  let promises: Array<Promise<undefined>> = [];
  for (let compressor of compressors) {
    promises.push(
      // @ts-expect-error TS2345
      runCompressor(
        compressor,
        cloneStream(stream),
        options,
        outputFS,
        fullPath,
        writeOptions,
        devDeps,
        api,
      ),
    );
  }

  await Promise.all(promises);
}

async function runCompressor(
  compressor: LoadedPlugin<Compressor>,
  // @ts-expect-error TS2503
  stream: stream.Readable,
  options: AtlaspackOptions,
  outputFS: FileSystem,
  filePath: FilePath,
  writeOptions: FileOptions | null | undefined,
  devDeps: Map<string, string>,
  api: RunAPI<PackagedBundleInfo>,
) {
  let measurement;
  try {
    measurement = tracer.createMeasurement(
      compressor.name,
      'compress',
      path.relative(options.projectRoot, filePath),
    );
    let res = await compressor.plugin.compress({
      stream,
      options: new PluginOptions(options),
      logger: new PluginLogger({origin: compressor.name}),
      tracer: new PluginTracer({origin: compressor.name, category: 'compress'}),
    });

    if (res != null) {
      await new Promise(
        (
          resolve: (result: Promise<undefined> | undefined) => void,
          reject: (error?: any) => void,
        ) =>
          pipeline(
            res.stream,
            outputFS.createWriteStream(
              filePath + (res.type != null ? '.' + res.type : ''),
              writeOptions,
            ),
            (err) => {
              if (err) reject(err);
              // @ts-expect-error TS2794
              else resolve();
            },
          ),
      );
    }
  } catch (err: any) {
    throw new ThrowableDiagnostic({
      diagnostic: errorToDiagnostic(err, {
        origin: compressor.name,
      }),
    });
  } finally {
    measurement && measurement.end();
    // Add dev deps for compressor plugins AFTER running them, to account for lazy require().
    let devDepRequest = await createDevDependency(
      {
        specifier: compressor.name,
        resolveFrom: compressor.resolveFrom,
      },
      devDeps,
      options,
    );
    await runDevDepRequest(api, devDepRequest);
  }
}

function advanceLineColumn(
  line: number,
  column: number,
  buf: Buffer,
): {line: number; column: number} {
  for (let i = 0; i < buf.length; i++) {
    if (buf[i] === 0x0a) {
      line++;
      column = 0;
    } else {
      column++;
    }
  }
  return {line, column};
}

function replaceStream(
  hashRefToNameHash: Map<string, string>,
  replacements?: HashRefReplacement[],
) {
  let boundaryStr = Buffer.alloc(0);
  let replaced = Buffer.alloc(0);
  let outputLine = 0;
  let outputColumn = 0;
  return new Transform({
    transform(
      chunk: Buffer | string,
      encoding: string,
      cb: (
        error?: Error | null | undefined,
        data?: Buffer | string | null | undefined,
      ) => void,
    ) {
      let str = Buffer.concat([boundaryStr, Buffer.from(chunk)]);
      let lastMatchI = 0;
      if (replaced.length < str.byteLength) {
        replaced = Buffer.alloc(str.byteLength);
      }
      let replacedLength = 0;

      while (lastMatchI < str.byteLength) {
        let matchI = str.indexOf(HASH_REF_PREFIX, lastMatchI);
        if (matchI === -1) {
          replaced.set(
            str.subarray(lastMatchI, str.byteLength),
            replacedLength,
          );
          replacedLength += str.byteLength - lastMatchI;
          break;
        } else {
          let match = str
            .subarray(matchI, matchI + HASH_REF_PREFIX_LEN + HASH_REF_HASH_LEN)
            .toString();
          let replacement = Buffer.from(hashRefToNameHash.get(match) ?? match);
          // Copy pre-match content FIRST so position calculation includes it
          replaced.set(str.subarray(lastMatchI, matchI), replacedLength);
          replacedLength += matchI - lastMatchI;
          if (replacements) {
            const pos = advanceLineColumn(
              outputLine,
              outputColumn,
              replaced.subarray(0, replacedLength),
            );
            replacements.push({
              line: pos.line,
              column: pos.column,
              originalLength: HASH_REF_PLACEHOLDER_LEN,
              newLength: replacement.byteLength,
            });
          }
          replaced.set(replacement, replacedLength);
          replacedLength += replacement.byteLength;
          lastMatchI = matchI + HASH_REF_PREFIX_LEN + HASH_REF_HASH_LEN;
        }
      }

      const pushLen = replacedLength - BOUNDARY_LENGTH;
      const pushed = advanceLineColumn(
        outputLine,
        outputColumn,
        replaced.subarray(0, pushLen),
      );
      outputLine = pushed.line;
      outputColumn = pushed.column;

      boundaryStr = replaced.subarray(
        replacedLength - BOUNDARY_LENGTH,
        replacedLength,
      );
      let strUpToBoundary = replaced.subarray(0, pushLen);
      cb(null, strUpToBoundary);
    },

    flush(
      cb: (
        error?: Error | null | undefined,
        data?: Buffer | string | null | undefined,
      ) => void,
    ) {
      cb(null, boundaryStr);
    },
  });
}

// @ts-expect-error TS2503
function cloneStream(readable: stream.Readable | stream.Transform) {
  let res = new Readable();
  res._read = () => {};
  // @ts-expect-error TS7006
  readable.on('data', (chunk) => res.push(chunk));
  readable.on('end', () => res.push(null));
  // @ts-expect-error TS7006
  readable.on('error', (err) => res.emit('error', err));
  return res;
}
