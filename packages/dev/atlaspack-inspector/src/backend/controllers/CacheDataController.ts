import {Router} from 'express';
import {getCacheStats} from '../services/getCacheStats';
import {
  getCache,
  getRequestTracker,
} from '../config/middleware/cacheDataMiddleware';

export function makeCacheDataController(): Router {
  const router = Router();

  router.get('/api/stats', (_req, res) => {
    const cache = getCache(res);
    const stats = getCacheStats(cache);
    res.json(stats);
  });

  router.get('/api/cache-invalidation-files/', (req, res) => {
    const limit = req.query.limit != null ? Number(req.query.limit) : 100;
    const cursor = req.query.cursor as string | undefined;
    const sortBy = req.query.sortBy as string;

    const requestTracker = getRequestTracker(res);
    // 0 is file
    let files = requestTracker.graph.nodes
      .filter((node: any) => node.type === 0)
      .map((node: any) => ({
        ...node,
        invalidationCount: requestTracker.graph
          .getNodeIdsConnectedTo(
            requestTracker.graph.getNodeIdByContentKey(node.id),
            -1,
          )
          // type 1 is request
          .filter((node: any) => requestTracker.graph.getNode(node).type === 1)
          .length,
      }));

    if (sortBy == null || sortBy === 'invalidationCount') {
      files.sort((a: any, b: any) => b.invalidationCount - a.invalidationCount);
    }

    const count = files.length;
    const cursorIndex =
      cursor != null && cursor !== ''
        ? files.findIndex((node: any) => node.id === cursor) + 1
        : 0;
    const resultFiles = files.slice(cursorIndex, cursorIndex + limit);

    res.json({
      files: resultFiles,
      count,
      nextPageCursor:
        cursorIndex + limit < count
          ? resultFiles[resultFiles.length - 1]
          : null,
      hasNextPage: cursorIndex + limit < count,
      limit,
    });
  });

  router.get('/api/cache-invalidation-files/:fileId', (req, res) => {
    const requestTracker = getRequestTracker(res);
    const file = requestTracker.graph.nodes.find(
      (node: any) => node.id === req.params.fileId,
    );
    if (!file) {
      res.status(400).json({error: 'File not found'});
      return;
    }

    const nodeId = requestTracker.graph.getNodeIdByContentKey(file.id);
    const nodes = requestTracker.graph.getNodeIdsConnectedTo(nodeId, -1);

    const invalidatedNodes = nodes
      .map((node: any) => requestTracker.graph.getNode(node))
      .filter((node: any) => node.type === 1);

    const response = {
      file,
      invalidatedNodesCount: invalidatedNodes.length,
    };

    res.json(response);
  });

  router.get(
    '/api/cache-invalidation-files/:fileId/invalidated-nodes',
    (req, res) => {
      const limit = req.query.limit != null ? Number(req.query.limit) : 100;
      const cursor = req.query.cursor as string | undefined;

      const requestTracker = getRequestTracker(res);
      const file = requestTracker.graph.nodes.find(
        (node: any) => node.id === req.params.fileId,
      );
      if (!file) {
        res.status(400).json({error: 'File not found'});
        return;
      }

      const nodeId = requestTracker.graph.getNodeIdByContentKey(file.id);
      const nodes = requestTracker.graph.getNodeIdsConnectedTo(nodeId, -1);

      const invalidatedNodes = nodes
        .map((node: any) => requestTracker.graph.getNode(node))
        .filter((node: any) => node.type === 1);
      const cursorIndex =
        cursor != null && cursor !== ''
          ? invalidatedNodes.findIndex((node: any) => node.id === cursor) + 1
          : 0;
      const resultNodes = invalidatedNodes.slice(
        cursorIndex,
        cursorIndex + limit,
      );

      const response = {
        data: resultNodes,
        count: invalidatedNodes.length,
        nextPageCursor:
          cursorIndex + limit < invalidatedNodes.length
            ? resultNodes[resultNodes.length - 1]
            : null,
        hasNextPage: cursorIndex + limit < invalidatedNodes.length,
        limit,
      };

      res.json(response);
    },
  );

  router.get('/api/cache-keys/', (req, res) => {
    const cache = getCache(res);
    const sortBy = req.query.sortBy as string;
    const limit = req.query.limit != null ? Number(req.query.limit) : 100;
    const cursor = req.query.cursor as string | undefined;

    let resultKeys = Array.from(cache.keys());
    if (sortBy === 'size') {
      const sizes = resultKeys.map((key: any) => [
        key,
        cache.getBlobSync(key).length,
      ]);
      sizes.sort((a, b) => (b[1] as number) - (a[1] as number));
      resultKeys = sizes.map(([key]: any) => key as string);
    }

    const count = resultKeys.length;
    const cursorIndex =
      cursor != null ? resultKeys.indexOf(cursor as string) + 1 : 0;
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
    const cache = getCache(res);
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
