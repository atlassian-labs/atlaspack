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
import {LazyValue} from './LazyValue';

/**
 * Represents the cache data loaded.
 *
 * The current implementation will only load or compute the data when these properties are read,
 * using {@link LazyValue}. This speeds-up starting up the inspector app UI, but the cache data
 * will be loaded when relevant endpoints are first requested.
 */
export interface CacheData {
  assetGraph: AssetGraph | null;
  bundleGraph: BundleGraph | null;
  treemap: Treemap | null;
  requestTracker: RequestTracker;
  cache: LMDBLiteCache;
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
  let requestGraphBlob: string | null = null;
  for (let key of cache.keys()) {
    if (key.startsWith('RequestTracker/') && key.endsWith('/RequestGraph')) {
      requestGraphBlob = key as string;
    }
  }

  if (!requestGraphBlob) {
    logger.warn('No request graph key found in cache');
    return {requestTracker: null};
  }

  const requestGraphKey = requestGraphBlob.split('/').slice(0, -1).join('/');
  try {
    logger.debug(
      {requestGraphBlob, requestGraphKey},
      'Loading RequestGraph...',
    );
    const {requestGraph} = await readAndDeserializeRequestGraph(
      cache,
      requestGraphBlob,
      requestGraphKey,
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
- The cache was built without the required 'cachePerformanceImprovements' feature flag set to true

The inspector will try to load caches even if there are version mismatches, but it might fail due to breaking
changes between versions.

You can run "atlaspack-inspector build <entrypoint...>" to build the app for the inspector with correct
feature-flags.`,
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

function getSync(cache: LMDBLiteCache, key: string): CacheResult | null {
  const blob = cache.getBlobSync(key);
  if (!blob) {
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

Make sure you have run "atlaspack build --feature-flag cachePerformanceImprovements=true" before running the inspector.

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

  const bundleGraphLazyValue = new LazyValue(() => {
    logger.info('Loading BundleGraph...');
    const bundleGraphRequest = requestTracker.graph.nodes.find(
      (node: RequestGraphNode) =>
        node &&
        node.type === 1 &&
        node.requestType === requestTypes.bundle_graph_request,
    );
    const bundleGraph =
      bundleGraphRequest?.resultCacheKey &&
      getSync(cache, bundleGraphRequest.resultCacheKey)?.bundleGraph;
    if (!bundleGraphRequest) {
      logger.error('Failed to find bundle graph request');
    } else if (bundleGraph != null) {
      logger.debug('Loaded BundleGraph');
    }
    return bundleGraph;
  });

  const treemap = new LazyValue(() => {
    logger.info({projectRoot}, 'Building treemap...');
    const bundleGraph = bundleGraphLazyValue.get();
    return bundleGraph && requestTracker
      ? buildTreemap({projectRoot, repositoryRoot, bundleGraph, requestTracker})
      : null;
  });

  const assetGraph = new LazyValue(() => {
    logger.info('Loading AssetGraph...');
    const assetGraphRequest = requestTracker.graph.nodes.find(
      (node: RequestGraphNode) =>
        node &&
        node.type === 1 &&
        node.requestType === requestTypes.asset_graph_request,
    );

    const assetGraph =
      assetGraphRequest?.resultCacheKey &&
      getSync(cache, assetGraphRequest.resultCacheKey)?.assetGraph;

    if (!assetGraphRequest) {
      logger.error('Failed to find asset graph request');
    } else if (assetGraph != null) {
      logger.debug('Loaded AssetGraph');
    }

    return assetGraph;
  });

  return {
    get assetGraph() {
      return assetGraph.get();
    },
    get bundleGraph() {
      return bundleGraphLazyValue.get();
    },
    get treemap() {
      return treemap.get();
    },
    cache,
    requestTracker,
  };
}
