/* eslint-disable monorepo/no-internal-import */
import path from 'path';
import v8 from 'v8';
import {restoreDeserializedObject} from '@atlaspack/build-cache';
import {LMDBLiteCache} from '@atlaspack/cache';
// @ts-expect-error TS2749
import type {CacheResult} from '@atlaspack/core/lib/types.js';
// @ts-expect-error TS2749
import type {RequestGraphNode} from '@atlaspack/core/lib/RequestTracker.js';
// @ts-expect-error TS2749
import RequestTracker from '@atlaspack/core/lib/RequestTracker.js';
import {version as atlaspackCoreVersion} from '@atlaspack/core/package.json';
// @ts-expect-error TS2749
import AssetGraph from '@atlaspack/core/lib/AssetGraph.js';
// @ts-expect-error TS2749
import BundleGraph from '@atlaspack/core/lib/BundleGraph.js';
import {
  requestTypes,
  readAndDeserializeRequestGraph,
  // @ts-expect-error TS2749
} from '@atlaspack/core/lib/RequestTracker.js';
import fs from 'fs';

import {buildTreemap, Treemap} from './buildTreemap';
import {logger} from '../config/logger';

/**
 * Represents the cache data loaded.
 */
export interface CacheData {
  assetGraph: AssetGraph | null;
  bundleGraph: BundleGraph | null;
  treemap: Treemap | null;
  requestTracker: RequestTracker;
  cache: LMDBLiteCache;
}

/**
 * Scans the cache directory for request graph files.
 *
 * The request graph is stored as FSCache large blob files on disk
 * (e.g., `requestGraph-${hash}-0`), not in LMDB. This function finds those
 * files and returns the keys needed to read them.
 */
function findRequestGraphInFilesystem(
  cacheDir: string,
): {requestGraphKey: string; cacheKey: string} | null {
  try {
    const files = fs.readdirSync(cacheDir);
    const requestGraphFile = files.find(
      (file) =>
        file.startsWith('requestGraph-') &&
        !file.startsWith('requestGraph-nodes-') &&
        file.endsWith('-0'),
    );
    if (!requestGraphFile) {
      return null;
    }
    // Strip the '-0' chunk suffix to recover the cache key
    const requestGraphKey = requestGraphFile.slice(0, -2);
    // Strip the 'requestGraph-' prefix to recover the cacheKey used for node keys
    const cacheKey = requestGraphKey.replace(/^requestGraph-/, '');
    return {requestGraphKey, cacheKey};
  } catch {
    return null;
  }
}

/**
 * Similar to `@atlaspack/query`.
 *
 * Since query is using a different approach to determine valid BundleGraph/AssetGraph in
 * the cache it was easier to rewrite this here.
 *
 * This will find the first `RequestGraph` entry in the cache and load it.
 *
 * We should be keeping meta-data about the builds that correspond to these entries so as
 * to load the most relevant one (for example the most recent) as well as performing
 * clean-up in other areas of the `atlaspack` codebase.
 */
export async function loadRequestTracker(cache: LMDBLiteCache): Promise<{
  requestTracker: RequestTracker | null;
}> {
  let requestGraphKey: string | null = null;
  let cacheKey: string | null = null;

  // Check LMDB for the request graph key (legacy path, kept for forward compatibility)
  for (let key of cache.keys()) {
    if (key.startsWith('RequestTracker/') && key.endsWith('/RequestGraph')) {
      requestGraphKey = key as string;
      cacheKey = (key as string).split('/').slice(0, -1).join('/');
      break;
    }
  }

  // The request graph is stored as FSCache large blob files on disk, not in LMDB
  if (!requestGraphKey) {
    const found = findRequestGraphInFilesystem(cache.dir);
    if (found) {
      requestGraphKey = found.requestGraphKey;
      cacheKey = found.cacheKey;
    }
  }

  if (!requestGraphKey || !cacheKey) {
    logger.warn('No request graph key found in cache');
    return {requestTracker: null};
  }

  try {
    logger.debug({requestGraphKey, cacheKey}, 'Loading RequestGraph...');
    const {requestGraph} = await readAndDeserializeRequestGraph(
      cache,
      requestGraphKey,
      cacheKey,
    );

    const requestTracker = new RequestTracker({
      graph: requestGraph,
      farm: null,
      options: null,
    });

    return {
      requestTracker,
    };
  } catch (e) {
    logger.error(
      e,
      `Error loading Request Graph.

A failure happened when loading atlaspack cache data.

This might happen because:

- There is a version mismatch between the atlaspack-inspector and the atlaspack CLI used to build the cache
The inspector will try to load caches even if there are version mismatches, but it might fail due to breaking
changes between versions.

You can run "atlaspack-inspector build <entrypoint...>" to build the app for the inspector.`,
    );
    return {
      requestTracker: null,
    };
  }
}

/**
 * Find the cache path from the `--target` flag.
 *
 * This is meant to make the tool nicer to use by forgiving a developer if they specify
 * `--target` as the path to their project root or to a sub-directory, instead of the
 * cache.
 *
 * It will also prevent creating caches where there were none, since opening a non-existent
 * cache would create a new empty cache.
 */
export function findCachePath(target: string): string | null {
  target = path.resolve(process.cwd(), target);

  if (fs.existsSync(path.join(target, 'data.mdb'))) {
    return target;
  }

  if (fs.existsSync(path.join(target, '.parcel-cache'))) {
    return path.join(target, '.parcel-cache');
  }

  if (fs.existsSync(path.join(target, '.atlaspack-cache'))) {
    return path.join(target, '.atlaspack-cache');
  }

  if (
    path.dirname(target) !== target &&
    target &&
    target !== '/' &&
    target !== '.'
  ) {
    return findCachePath(path.dirname(target));
  }

  return null;
}

function mapObjectStringValues(
  object: any,
  fn: (key: string, value: any) => any,
): any {
  if (typeof object !== 'object' || object === null) {
    return object;
  }

  if (Array.isArray(object)) {
    return object;
  }

  return Object.fromEntries(
    Object.entries(object).map(([key, value]) => {
      if (typeof value === 'string') {
        return [key, fn(key, value)];
      }

      if (
        value &&
        typeof value === 'object' &&
        value.constructor.name !== 'Object'
      ) {
        return [key, value];
      }

      return [key, mapObjectStringValues(value, fn)];
    }),
  );
}

async function getAsync(
  cache: LMDBLiteCache,
  key: string,
): Promise<CacheResult | null> {
  // Try LMDB first (legacy path, kept for forward compatibility)
  let blob: Buffer | null | undefined = await cache.getBuffer(key);

  // The cache stores request results as FSCache large blob files on disk
  if (blob == null) {
    const hasBlob = await cache.hasLargeBlob(key);
    if (hasBlob) {
      blob = await cache.getLargeBlob(key);
    }
  }

  if (!blob || blob.length === 0) {
    return null;
  }

  let deserialized = v8.deserialize(blob);
  deserialized = mapObjectStringValues(deserialized, (key, value) => {
    if (key === '$$type') {
      return value.replace(/^\d+\.\d+\.\d+:/, `${atlaspackCoreVersion}:`);
    }
    return value;
  });

  return restoreDeserializedObject(deserialized) as CacheResult;
}

/**
 * Loads the cache and pre-processes some data.
 */
export async function loadCacheData({
  target,
  projectRoot,
  repositoryRoot,
}: {
  target: string;
  projectRoot: string;
  repositoryRoot: string;
}): Promise<CacheData> {
  const cachePath = findCachePath(target);
  if (!cachePath) {
    throw new Error(
      `Invalid cache path provided, could not find cache in any directory above the path provided: ${target}.

Alternatively you can run "atlaspack-inspector build <entrypoint...>" to build the app for the inspector.`,
    );
  }

  logger.info({cachePath, projectRoot}, 'Loading graphs...');
  const cache = new LMDBLiteCache(cachePath);

  const {requestTracker} = await loadRequestTracker(cache);
  if (!requestTracker) {
    throw new Error('Failed to load request tracker');
  }
  logger.debug('Loaded RequestTracker');

  logger.info('Loading BundleGraph...');
  const bundleGraphRequest = requestTracker.graph.nodes.find(
    (node: RequestGraphNode) =>
      node &&
      node.type === 1 &&
      node.requestType === requestTypes.bundle_graph_request,
  );
  const bundleGraphResult =
    bundleGraphRequest?.resultCacheKey != null
      ? await getAsync(cache, bundleGraphRequest.resultCacheKey)
      : null;
  const bundleGraph = bundleGraphResult?.bundleGraph ?? null;
  if (!bundleGraphRequest) {
    logger.error('Failed to find bundle graph request');
  } else if (bundleGraph != null) {
    logger.debug('Loaded BundleGraph');
  }

  logger.info({projectRoot}, 'Building treemap...');
  const treemap =
    bundleGraph && requestTracker
      ? buildTreemap({projectRoot, repositoryRoot, bundleGraph, requestTracker})
      : null;

  logger.info('Loading AssetGraph...');
  const assetGraphRequest = requestTracker.graph.nodes.find(
    (node: RequestGraphNode) =>
      node &&
      node.type === 1 &&
      node.requestType === requestTypes.asset_graph_request,
  );
  const assetGraphResult =
    assetGraphRequest?.resultCacheKey != null
      ? await getAsync(cache, assetGraphRequest.resultCacheKey)
      : null;
  const assetGraph = assetGraphResult?.assetGraph ?? null;
  if (!assetGraphRequest) {
    logger.error('Failed to find asset graph request');
  } else if (assetGraph != null) {
    logger.debug('Loaded AssetGraph');
  }

  return {
    assetGraph,
    bundleGraph,
    treemap,
    cache,
    requestTracker,
  };
}
