/* eslint-disable monorepo/no-internal-import */
import {Request, Response, NextFunction} from 'express';
import {CacheData} from '../../services/loadCacheData';
import {HTTPError} from '../../errors/HTTPError';
// @ts-expect-error TS2749
import BundleGraph from '@atlaspack/core/lib/BundleGraph.js';
import {Treemap} from '../../services/buildTreemap';
// @ts-expect-error TS2749
import RequestTracker from '@atlaspack/core/lib/RequestTracker.js';
// @ts-expect-error TS2749
import AssetGraph from '@atlaspack/core/lib/AssetGraph.js';
import {LMDBLiteCache} from '@atlaspack/cache';

export function getCacheDataKey<K extends keyof CacheData>(
  res: Response,
  key: K,
  missingKeyError?: string,
): NonNullable<CacheData[K]> {
  const cacheData: CacheData = res.locals.cacheData;
  if (!cacheData[key]) {
    throw new HTTPError(missingKeyError ?? `${key} not found`, 500);
  }
  return cacheData[key];
}

export const getCache = (res: Response): LMDBLiteCache =>
  getCacheDataKey(res, 'cache');

let missingErrorHelp = `
Please make sure to run "atlaspack build" before running the inspector.
Or your application's build command.
`.trim();

export const getBundleGraph = (res: Response): BundleGraph =>
  getCacheDataKey(
    res,
    'bundleGraph',
    `
BundleGraph not found in cache. The bundle graph is the data structure that contains all the information about the
bundles in the application.

This could happen if you are trying to run the inspector against a cache which has no builds or where a build terminated
with errors before bundling.

${missingErrorHelp}`.trim(),
  );

export const getTreemap = (res: Response): Treemap =>
  getCacheDataKey(
    res,
    'treemap',
    `
Treemap not found in cache. The treemap data can only be generated if the cache contains a bundle graph and request tracker.

This could happen if you are trying to run the inspector against a cache which has no builds or where a build terminated
with errors before writing the cache or finishing bundling.

${missingErrorHelp}`.trim(),
  );

export const getRequestTracker = (res: Response): RequestTracker =>
  getCacheDataKey(
    res,
    'requestTracker',
    `
RequestTracker not found in cache. The request tracker is the data structure that contains all the information about the
build tasks made during the build.

This could happen if you are trying to run the inspector against a cache which has no builds or where a build terminated
with errors before writing the cache.

${missingErrorHelp}`.trim(),
  );

export const getAssetGraph = (res: Response): AssetGraph =>
  getCacheDataKey(
    res,
    'assetGraph',
    `
AssetGraph not found in cache. The asset graph is the data structure that contains all the information about the
assets in the application.

This could happen if you are trying to run the inspector against a cache which has no builds or where a build terminated
with errors before transforming and resolving assets.

${missingErrorHelp}`.trim(),
  );

export function cacheDataMiddleware(cacheData: Promise<CacheData>) {
  return async (_req: Request, res: Response, next: NextFunction) => {
    res.locals.cacheData = await cacheData;
    next();
  };
}
