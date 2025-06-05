import {Request, Response, NextFunction} from 'express';
import {CacheData} from '../../services/loadCacheData';
import {HTTPError} from '../../errors/HTTPError';
import BundleGraph from '@atlaspack/core/lib/BundleGraph.js';
import {Treemap} from '../../services/buildTreemap';
import RequestTracker from '@atlaspack/core/lib/RequestTracker.js';
import AssetGraph from '@atlaspack/core/lib/AssetGraph.js';

export function getCacheDataKey<K extends keyof CacheData>(
  res: Response,
  key: K,
): NonNullable<CacheData[K]> {
  const cacheData: CacheData = res.locals.cacheData;
  if (!cacheData[key]) {
    throw new HTTPError(`${key} not found`, 500);
  }
  return cacheData[key];
}

export const getBundleGraph = (res: Response): BundleGraph =>
  getCacheDataKey(res, 'bundleGraph');

export const getTreemap = (res: Response): Treemap =>
  getCacheDataKey(res, 'treemap');

export const getRequestTracker = (res: Response): RequestTracker =>
  getCacheDataKey(res, 'requestTracker');

export const getAssetGraph = (res: Response): AssetGraph =>
  getCacheDataKey(res, 'assetGraph');

export function cacheDataMiddleware(cacheData: CacheData) {
  return (_req: Request, res: Response, next: NextFunction) => {
    res.locals.cacheData = cacheData;
    next();
  };
}
