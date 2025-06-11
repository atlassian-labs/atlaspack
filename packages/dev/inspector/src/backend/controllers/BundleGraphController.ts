import path from 'path';
import fs from 'fs';

import {Router} from 'express';
import {requestTypes} from '@atlaspack/core/lib/RequestTracker.js';
import {RequestGraphNode} from '@atlaspack/core/lib/RequestTracker.js';
import {Node} from '@atlaspack/core/lib/types.js';

import {buildJsonGraph} from '../services/buildJsonGraph';
import {
  getBundleGraph,
  getRequestTracker,
  getTreemap,
} from '../config/middleware/cacheDataMiddleware';
import {
  getWriteBundleRequestsByBundleId,
  Treemap,
} from '../services/buildTreemap';

export interface BundleGraphControllerParams {
  projectRoot: string;
}

export function makeBundleGraphController({
  projectRoot,
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
        const filePath = writeBundleRequest?.result?.filePath
          ? path.join(projectRoot, writeBundleRequest.result.filePath)
          : null;
        const size = filePath ? fs.statSync(filePath).size : 0;
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
    const childBundles = bundleGraph.getChildBundles(bundle).map((b) => ({
      id: b.id,
    }));

    res.json({
      childBundles,
    });
  });

  return router;
}
