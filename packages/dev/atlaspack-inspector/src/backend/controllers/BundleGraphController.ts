/* eslint-disable monorepo/no-internal-import */
import {Router} from 'express';
// @ts-expect-error TS2749
import {Node} from '@atlaspack/core/lib/types.js';

import {buildJsonGraph} from '../services/buildJsonGraph';
import {
  getBundleGraph,
  getRequestTracker,
} from '../config/middleware/cacheDataMiddleware';
import {
  findBundleInfo,
  getWriteBundleRequestsByBundleId,
} from '../services/buildTreemap';

export interface BundleGraphControllerParams {
  projectRoot: string;
  repositoryRoot: string;
}

export function makeBundleGraphController({
  projectRoot,
  repositoryRoot,
}: BundleGraphControllerParams): Router {
  const router = Router();

  router.get('/api/bundle-graph', (req, res) => {
    const bundleGraph = getBundleGraph(res);
    const requestTracker = getRequestTracker(res);

    const writeBundleRequestsByBundleId =
      getWriteBundleRequestsByBundleId(requestTracker);

    const jsonGraph = buildJsonGraph(
      bundleGraph._graph,
      typeof req.query.rootNodeId === 'string' ? req.query.rootNodeId : null,
      (node: Node) => node.type !== 'asset' && node.type !== 'dependency',
      (node: Node) => {
        const writeBundleRequest = writeBundleRequestsByBundleId.get(node.id);
        const {size, filePath} = findBundleInfo({
          writeBundleRequest,
          projectRoot,
          repositoryRoot,
          node,
        });
        return {
          size,
          filePath,
        };
      },
    );

    res.json(jsonGraph);
  });

  router.get('/api/bundle-graph/related-bundles', (req, res) => {
    const bundleGraph = getBundleGraph(res);
    const bundleId = req.query.bundle as string;

    const bundle = bundleGraph._graph.getNode(
      bundleGraph._graph.getNodeIdByContentKey(bundleId),
    );
    const childBundles = bundleGraph.getChildBundles(bundle).map((b: any) => ({
      id: b.id,
    }));

    res.json({
      childBundles,
    });
  });

  return router;
}
