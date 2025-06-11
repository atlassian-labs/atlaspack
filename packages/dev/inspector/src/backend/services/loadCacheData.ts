import path from 'path';
import {LMDBLiteCache} from '@atlaspack/cache';
import type {CacheResult} from '@atlaspack/core/lib/types.js';
import type {RequestGraphNode} from '@atlaspack/core/lib/RequestTracker.js';
import RequestTracker from '@atlaspack/core/lib/RequestTracker.js';
import AssetGraph from '@atlaspack/core/lib/AssetGraph.js';
import BundleGraph from '@atlaspack/core/lib/BundleGraph.js';
import {
  requestTypes,
  readAndDeserializeRequestGraph,
} from '@atlaspack/core/lib/RequestTracker.js';
import fs from 'fs';

import {buildTreemap, Treemap} from './buildTreemap';
import {logger} from '../config/logger';

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
    const {requestGraph} = await readAndDeserializeRequestGraph(
      cache,
      requestGraphBlob,
      requestGraphKey,
    );

    // @ts-expect-error
    const requestTracker = new RequestTracker({
      graph: requestGraph,
      farm: null,
      options: null,
    });

    return {
      requestTracker,
    };
  } catch (e) {
    logger.error(e, 'Error loading Request Graph');
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

/**
 * Loads the cache and pre-processes some data.
 */
export async function loadCacheData(
  target: string,
  projectRoot: string,
): Promise<CacheData> {
  const cachePath = findCachePath(target);
  if (!cachePath) {
    throw new Error(
      'Invalid cache path provided, could not find cache in any directory above the path provided',
    );
  }
  const cache = new LMDBLiteCache(cachePath);

  logger.info({cachePath, projectRoot}, 'Loading graphs...');
  const {requestTracker} = await loadRequestTracker(cache);
  if (!requestTracker) {
    throw new Error('Failed to load request tracker');
  }
  logger.info('Loaded RequestTracker');

  const assetGraphRequest = requestTracker.graph.nodes.find(
    (node: RequestGraphNode) =>
      node &&
      node.type === 1 &&
      node.requestType === requestTypes.asset_graph_request,
  );

  const assetGraph =
    assetGraphRequest?.resultCacheKey &&
    ((await cache.get(assetGraphRequest.resultCacheKey)) as CacheResult)
      ?.assetGraph;

  if (!assetGraphRequest) {
    logger.error('Failed to find asset graph request');
  } else if (assetGraph != null) {
    logger.info('Loaded AssetGraph');
  }

  const bundleGraphRequest = requestTracker.graph.nodes.find(
    (node: RequestGraphNode) =>
      node &&
      node.type === 1 &&
      node.requestType === requestTypes.bundle_graph_request,
  );
  const bundleGraph =
    bundleGraphRequest?.resultCacheKey &&
    ((await cache.get(bundleGraphRequest.resultCacheKey)) as CacheResult)
      ?.bundleGraph;
  if (!bundleGraphRequest) {
    logger.error('Failed to find bundle graph request');
  } else if (bundleGraph != null) {
    logger.info('Loaded BundleGraph');
  }

  logger.info('Building treemap...');
  const treemap =
    bundleGraph && requestTracker
      ? buildTreemap(projectRoot, bundleGraph, requestTracker)
      : null;

  return {
    assetGraph,
    bundleGraph,
    treemap,
    cache,
    requestTracker,
  };
}
