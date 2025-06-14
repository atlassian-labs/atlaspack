// @flow strict-local

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

const HASH_REF_PREFIX_LEN = HASH_REF_PREFIX.length;
const BOUNDARY_LENGTH = HASH_REF_PREFIX.length + 32 - 1;

type WriteBundleRequestInput = {|
  bundleGraph: BundleGraph,
  bundle: Bundle,
  info: BundleInfo,
  hashRefToNameHash: Map<string, string>,
|};

export type WriteBundleRequestResult = PackagedBundleInfo;

type RunInput<TResult> = {|
  input: WriteBundleRequestInput,
  ...StaticRunOpts<TResult>,
|};

export type WriteBundleRequest = {|
  id: ContentKey,
  +type: typeof requestTypes.write_bundle_request,
  run: (RunInput<PackagedBundleInfo>) => Async<PackagedBundleInfo>,
  input: WriteBundleRequestInput,
|};

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
    new TapStream((buf) => {
      size += buf.length;
    }),
  );

  let configResult = nullthrows(
    await api.runRequest<null, ConfigAndCachePath>(
      createAtlaspackConfigRequest(),
    ),
  );
  let config = getCachedAtlaspackConfig(configResult, options);

  let {devDeps, invalidDevDeps} = await getDevDepRequests(api);
  invalidateDevDeps(invalidDevDeps, options, config);

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
  );

  const hasSourceMap = getFeatureFlag('cachePerformanceImprovements')
    ? await options.cache.hasLargeBlob(mapKey)
    : await options.cache.has(mapKey);
  if (mapKey && env.sourceMap && !env.sourceMap.inline && hasSourceMap) {
    const mapEntry = getFeatureFlag('cachePerformanceImprovements')
      ? await options.cache.getLargeBlob(mapKey)
      : await options.cache.getBlob(mapKey);
    await writeFiles(
      blobToStream(mapEntry),
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

async function writeFiles(
  inputStream: stream$Readable,
  info: BundleInfo,
  hashRefToNameHash: Map<string, string>,
  options: AtlaspackOptions,
  config: AtlaspackConfig,
  outputFS: FileSystem,
  filePath: ProjectPath,
  writeOptions: ?FileOptions,
  devDeps: Map<string, string>,
  api: RunAPI<PackagedBundleInfo>,
) {
  let compressors = await config.getCompressors(
    fromProjectPathRelative(filePath),
  );
  let fullPath = fromProjectPath(options.projectRoot, filePath);

  let stream = info.hashReferences.length
    ? inputStream.pipe(replaceStream(hashRefToNameHash))
    : inputStream;

  let promises = [];
  for (let compressor of compressors) {
    promises.push(
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
  stream: stream$Readable,
  options: AtlaspackOptions,
  outputFS: FileSystem,
  filePath: FilePath,
  writeOptions: ?FileOptions,
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
      await new Promise((resolve, reject) =>
        pipeline(
          res.stream,
          outputFS.createWriteStream(
            filePath + (res.type != null ? '.' + res.type : ''),
            writeOptions,
          ),
          (err) => {
            if (err) reject(err);
            else resolve();
          },
        ),
      );
    }
  } catch (err) {
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

function replaceStream(hashRefToNameHash) {
  let boundaryStr = Buffer.alloc(0);
  let replaced = Buffer.alloc(0);
  return new Transform({
    transform(chunk, encoding, cb) {
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
          replaced.set(str.subarray(lastMatchI, matchI), replacedLength);
          replacedLength += matchI - lastMatchI;
          replaced.set(replacement, replacedLength);
          replacedLength += replacement.byteLength;
          lastMatchI = matchI + HASH_REF_PREFIX_LEN + HASH_REF_HASH_LEN;
        }
      }

      boundaryStr = replaced.subarray(
        replacedLength - BOUNDARY_LENGTH,
        replacedLength,
      );
      let strUpToBoundary = replaced.subarray(
        0,
        replacedLength - BOUNDARY_LENGTH,
      );
      cb(null, strUpToBoundary);
    },

    flush(cb) {
      cb(null, boundaryStr);
    },
  });
}

function cloneStream(readable) {
  let res = new Readable();
  // $FlowFixMe
  res._read = () => {};
  readable.on('data', (chunk) => res.push(chunk));
  readable.on('end', () => res.push(null));
  readable.on('error', (err) => res.emit('error', err));
  return res;
}
