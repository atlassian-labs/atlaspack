import {Router} from 'express';
import {getCacheStats} from '../services/getCacheStats';
import {getCacheDataKey} from '../config/middleware/cacheDataMiddleware';

export function makeCacheDataController(): Router {
  const router = Router();

  router.get('/api/stats', (_req, res) => {
    const cache = getCacheDataKey(res, 'cache');
    const stats = getCacheStats(cache);
    res.json(stats);
  });

  router.get('/api/cache-keys/', (req, res) => {
    const cache = getCacheDataKey(res, 'cache');
    const sortBy = req.query.sortBy as string;
    const limit = req.query.limit != null ? Number(req.query.limit) : 100;
    const cursor = req.query.cursor as string | undefined;

    let resultKeys = Array.from(cache.keys());
    if (sortBy === 'size') {
      const sizes = resultKeys.map((key) => [
        key,
        cache.getBlobSync(key).length,
      ]);
      sizes.sort((a, b) => (b[1] as number) - (a[1] as number));
      resultKeys = sizes.map(([key]) => key as string);
    }

    const count = resultKeys.length;
    const cursorIndex = cursor != null ? resultKeys.indexOf(cursor) + 1 : 0;
    resultKeys = resultKeys.slice(cursorIndex, cursorIndex + limit);

    res.json({
      keys: resultKeys,
      count,
      nextPageCursor:
        cursorIndex + limit < count ? resultKeys[resultKeys.length - 1] : null,
      hasNextPage: cursorIndex + limit < count,
      limit,
    });
  });

  router.get('/api/cache-value/:key', (req, res) => {
    const cache = getCacheDataKey(res, 'cache');
    const key = req.params.key;

    try {
      const value = cache.getBlobSync(key);
      // bigger than 1MB
      if (typeof value.length === 'number' && value.length > 1024 * 1024) {
        res.json({
          size: value.length,
          value: value.slice(0, 1024 * 1024).toString('utf-8'),
        });
        return;
      }

      res.json({
        size: value.length,
        value: value.toString('utf-8'),
      });
    } catch (error) {
      res.status(500).json({error: (error as Error).message});
    }
  });

  return router;
}
