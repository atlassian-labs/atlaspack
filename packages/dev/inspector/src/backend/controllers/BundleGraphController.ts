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

export interface BundleGraphControllerParams {
  projectRoot: string;
}

export function makeBundleGraphController({
  projectRoot,
}: BundleGraphControllerParams): Router {
  const router = Router();

  router.get('/api/bundle-graph/bundles', (req, res) => {
    const treemap = getTreemap(res);
    const bundles = treemap.bundles.map((bundle) => ({
      id: bundle.id,
      displayName: bundle.displayName,
      size: bundle.size,
      filePath: bundle.filePath,
    }));

    res.json({bundles, count: bundles.length});
  });

  router.get('/api/bundle-graph', (req, res) => {
    const bundleGraph = getBundleGraph(res);
    const requestTracker = getRequestTracker(res);

    const writeBundleRequests: any[] = requestTracker.graph.nodes.filter(
      (requestNode: RequestGraphNode) =>
        requestNode &&
        requestNode.type === 1 &&
        requestNode.requestType === requestTypes.write_bundle_request,
    );
    const writeBundleRequestsByBundleId = new Map(
      writeBundleRequests.map((request) => [request.result?.bundleId, request]),
    );

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

  router.get('/api/bundle-graph/node/:nodeId', (req, res) => {
    const nodeId = req.params.nodeId;
    const bundleGraph = getBundleGraph(res);
    const requestTracker = getRequestTracker(res);

    const node = bundleGraph._graph.getNode(
      bundleGraph._graph.getNodeIdByContentKey(nodeId),
    );
    const writeBundleRequest = requestTracker.graph.nodes.find(
      (requestNode: RequestGraphNode) =>
        requestNode &&
        requestNode.type === 1 &&
        requestNode.requestType === requestTypes.write_bundle_request &&
        requestNode.result?.bundleId === node.id,
    );

    res.json({
      bundleGraphNode: node,
      writeBundleRequest,
    });
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
