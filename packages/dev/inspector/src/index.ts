// @ts-check

// Stateful import to ensure serializers are loaded
require('@atlaspack/core');

import {program} from 'commander';
import fs from 'fs';
import {LMDBLiteCache} from '@atlaspack/cache';
// @ts-ignore
import {loadGraphs} from '@atlaspack/query';
import {requestTypes} from '@atlaspack/core/src/RequestTracker.js';
// @ts-ignore
import express, {Request, Response, NextFunction} from 'express';
// @ts-ignore
import cors from 'cors';
// @ts-ignore
import {ALL_EDGE_TYPES} from '@atlaspack/graph';
import type {
  BundleGraph,
  RequestTracker,
  Node,
  RequestGraphNode,
  CacheResult,
} from '@atlaspack/core';
import path from 'path';

const {
  setFeatureFlags,
  DEFAULT_FEATURE_FLAGS,
  // @ts-ignore
} = require('@atlaspack/feature-flags/src/index.js');
// We must import feature-flags that atlaspack-query will use (either lib or src)

interface JsonGraph<T> {
  nodes: Array<{
    id: string;
    nodeId: string;
    path: string[] | null;
    displayName: string;
    level: number;
    edges: string[];
    extra: T | null;
  }>;
}

interface CacheStats {
  size: number;
  count: number;
  keySize: number;
  assetContentCount: number;
  assetContentSize: number;
  assetMapCount: number;
  assetMapSize: number;
}

type AssetTreeNode = {
  children: Record<string, AssetTreeNode>;
  size: number;
  path: string;
};

interface Treemap {
  bundles: Array<{
    id: string;
    displayName: string;
    bundle: any;
    size: number;
    filePath: string;
    assetTree: AssetTreeNode;
  }>;
  totalSize: number;
}

function getDisplayName(node: Node): string {
  if (node.type === 'asset') {
    return `asset: ${node.value.filePath}`;
  }
  if (node.type === 'dependency') {
    return `dependency: import '${node.value.specifier}'`;
  }
  if (node.type === 'asset_group') {
    return `asset group: ${node.value.filePath}`;
  }
  if (node.type === 'bundle') {
    return `bundle: ${node.value.displayName}`;
  }

  return node.id;
}

function buildJsonGraph<T>(
  graph: BundleGraph['_graph'],
  rootNodeId: string | null,
  filter: (node: Node) => boolean = () => true,
  extra?: (node: Node) => T,
): JsonGraph<T> {
  const depths = new Map<string, number>();

  rootNodeId = rootNodeId
    ? graph.getNodeIdByContentKey(rootNodeId)
    : graph.rootNodeId;

  // build path from `graph.rootNodeId` node to `rootNodeId`
  let path: string[] | null = null;
  {
    const stack: string[] = [];
    const visited = new Set<string>();
    graph.dfs({
      visit: {
        enter(nodeId: string, context: any, actions: {stop: () => void}) {
          stack.push(nodeId);
          visited.add(nodeId);

          if (nodeId === rootNodeId) {
            actions.stop();
            path = stack.slice();
          }
        },
        exit() {
          stack.pop();
        },
      },
      getChildren: (nodeId: string) => {
        return graph.getNodeIdsConnectedFrom(nodeId, ALL_EDGE_TYPES);
      },
      startNodeId: graph.rootNodeId,
    });
  }

  // Build depths of all nodes in the graph
  {
    const topologicalOrder: string[] = [];
    graph.traverse(
      {
        exit(nodeId: string) {
          topologicalOrder.push(nodeId);
        },
      },
      rootNodeId,
      ALL_EDGE_TYPES,
    );
    topologicalOrder.reverse();

    for (let i = 0; i < topologicalOrder.length; i++) {
      const nodeId = topologicalOrder[i];
      const currentDepth = depths.get(nodeId) ?? 0;
      const neighbors = graph.getNodeIdsConnectedFrom(nodeId, ALL_EDGE_TYPES);
      for (const other of neighbors) {
        const increment = filter(graph.getNode(other)) ? 1 : 0;
        depths.set(
          other,
          Math.max(currentDepth + increment, depths.get(other) ?? 0),
        );
      }
    }
  }

  const allNodes: string[] = [];
  graph.traverse(
    (nodeId: string) => {
      allNodes.push(nodeId);
    },
    rootNodeId,
    ALL_EDGE_TYPES,
  );

  const maxDepth = 100;
  return {
    nodes: allNodes
      .filter((nodeId) => (depths.get(nodeId) ?? 0) < maxDepth)
      .filter((nodeId) => filter(graph.getNode(nodeId)))
      .map((nodeId) => ({
        id: graph.getNode(nodeId).id,
        nodeId,
        path:
          nodeId === rootNodeId
            ? path?.map((id) => graph.getNode(id).id) ?? null
            : null,
        displayName: getDisplayName(graph.getNode(nodeId)),
        level: depths.get(nodeId) ?? 0,
        edges: graph
          .getNodeIdsConnectedFrom(nodeId, ALL_EDGE_TYPES)
          .map((nodeId) => graph.getNode(nodeId).id),
        extra: extra ? extra(graph.getNode(nodeId)) : null,
      })),
  };
}

function getCacheStats(cache: LMDBLiteCache): CacheStats {
  const stats: CacheStats = {
    size: 0,
    count: 0,
    keySize: 0,
    assetContentCount: 0,
    assetContentSize: 0,
    assetMapCount: 0,
    assetMapSize: 0,
  };

  for (const key of cache.keys()) {
    const value = cache.getBlobSync(key);
    stats.size += value.length;
    stats.keySize += Buffer.from(key).length;
    stats.count++;
    if (key.endsWith(':content')) {
      stats.assetContentCount++;
      stats.assetContentSize += value.length;
    } else if (key.endsWith(':map')) {
      stats.assetMapCount++;
      stats.assetMapSize += value.length;
    }
  }

  return stats;
}

function buildTreemap(
  projectRoot: string,
  bundleGraph: BundleGraph,
  requestTracker: RequestTracker,
): Treemap {
  const treemap: Treemap = {
    bundles: [],
    totalSize: 0,
  };

  const writeBundleRequests = requestTracker.graph.nodes.filter(
    (requestNode: RequestGraphNode) =>
      requestNode &&
      requestNode.type === 1 &&
      requestNode.requestType === requestTypes.write_bundle_request,
  );
  const writeBundleRequestsByBundleId = new Map(
    writeBundleRequests.map((request) => [request.result?.bundleId, request]),
  );

  bundleGraph._graph.nodes.forEach((node: Node) => {
    if (node.type === 'bundle') {
      const writeBundleRequest = writeBundleRequestsByBundleId.get(node.id);
      const filePath = writeBundleRequest?.result?.filePath
        ? path.join(projectRoot, writeBundleRequest.result.filePath)
        : null;
      const size = filePath ? fs.statSync(filePath).size : 0;

      // this is the directory structure
      const assetTree = {path: '', children: {}, size: 0};

      const assets: any[] = [];
      // @ts-ignore
      bundleGraph.traverseAssets(node.value, (asset) => {
        assets.push(asset);
      });

      for (const asset of assets) {
        const filePath = asset.filePath;
        const parts = filePath.split('/');
        const assetSize = asset.stats.size;
        let current = assetTree;
        for (let part of parts) {
          current.children[part] = current.children[part] ?? {
            path: parts.slice(0, parts.indexOf(part) + 1).join('/'),
            children: {},
            size: 0,
          };
          current.children[part].size += assetSize;
          current.size += assetSize;
          current = current.children[part];
        }
      }

      treemap.bundles.push({
        id: node.id,
        displayName: node.value.displayName,
        bundle: node,
        size,
        filePath: filePath ?? '',
        assetTree,
      });

      treemap.totalSize += size;
    }
  });

  return treemap;
}

async function main() {
  const command = program
    .requiredOption('-t, --target <path>', 'Path to the target cache')
    .parse(process.argv);

  setFeatureFlags({
    ...DEFAULT_FEATURE_FLAGS,
    cachePerformanceImprovements: true,
  });

  const options = command.opts();
  const cache = new LMDBLiteCache(options.target);

  console.log('Loading graphs...');
  const {requestTracker /*, bundleGraph */} = await loadGraphs(options.target);
  if (!requestTracker) {
    throw new Error('Failed to load request tracker');
  }
  console.log('Loaded RequestTracker');

  const assetGraphRequest = requestTracker.graph.nodes.find(
    (node: RequestGraphNode) =>
      node &&
      node.type === 1 &&
      node.requestType === requestTypes.asset_graph_request,
  );
  const assetGraph =
    assetGraphRequest.resultCacheKey &&
    ((await cache.get(assetGraphRequest.resultCacheKey)) as CacheResult)
      ?.assetGraph;

  if (!assetGraphRequest) {
    console.error('Failed to find asset graph request');
  } else if (assetGraph != null) {
    console.log('Loaded AssetGraph');
  }
  const bundleGraphRequest = requestTracker.graph.nodes.find(
    (node: RequestGraphNode) =>
      node &&
      node.type === 1 &&
      node.requestType === requestTypes.bundle_graph_request,
  );
  const bundleGraph =
    bundleGraphRequest.resultCacheKey &&
    ((await cache.get(bundleGraphRequest.resultCacheKey)) as CacheResult)
      ?.bundleGraph;
  if (!bundleGraphRequest) {
    console.error('Failed to find bundle graph request');
  } else if (bundleGraph != null) {
    console.log('Loaded BundleGraph');
  }

  console.log('Building treemap...');
  const treemap =
    bundleGraph && requestTracker
      ? buildTreemap(path.dirname(options.target), bundleGraph, requestTracker)
      : null;

  const keys = Array.from(cache.keys());
  const projectRoot = path.dirname(options.target);

  const app = express();

  app.use(
    cors({
      // origin: 'http://localhost:3333',
      credentials: true,
    }),
  );

  app.use((req: Request, res: Response, next: NextFunction) => {
    if (res.headersSent) {
      console.log(req.method, req.url, res.statusCode);
    } else {
      res.on('finish', function () {
        console.log(req.method, req.url, res.statusCode);
      });
    }
    next();
  });

  app.get('/api/bundle-graph/bundles', (req, res) => {
    if (!bundleGraph) {
      res.status(500).json({error: 'Bundle graph not found'});
      return;
    }

    const bundles = (treemap?.bundles ?? []).map((bundle) => ({
      id: bundle.id,
      displayName: bundle.displayName,
      size: bundle.size,
      filePath: bundle.filePath,
    }));

    res.json({bundles, count: bundles.length});
  });

  app.get('/api/bundle-graph', (req: Request, res: Response) => {
    if (!bundleGraph) {
      res.status(500).json({error: 'Bundle graph not found'});
      return;
    }

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
        console.log(writeBundleRequest);
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

  app.get('/api/bundle-graph/node/:nodeId', (req: Request, res: Response) => {
    const nodeId = req.params.nodeId;
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

  app.get('/api/treemap', (req: Request, res: Response) => {
    if (!treemap) {
      res.status(500).json({error: 'Treemap not found'});
    }

    const limit = Number(req.query.limit ?? 10000);
    const offset = Number(req.query.offset ?? 0);

    const bundleId = req.query.bundle as string | null;
    let bundles = treemap!.bundles;
    if (bundleId) {
      bundles = bundles.filter((bundle) => bundle.id === bundleId);
    } else {
      bundles = bundles.map((bundle) => ({
        ...bundle,
        assetTree: {
          path: '',
          children: {},
          size: bundle.size,
        },
      }));
    }

    res.json({
      bundles: bundles.slice(offset, offset + limit),
      next: offset + limit < bundles.length ? offset + limit : null,
      count: bundles.length,
      totalSize: treemap!.totalSize,
    });
  });

  app.get('/api/asset-graph', (_req: Request, res: Response) => {
    // assetGraph does not have the full _graph interface, so we can't use buildJsonGraph here
    // Instead, just return a simple node list for now
    res.json({nodes: []});
  });

  app.get('/api/asset-graph/node/:nodeId', (req: Request, res: Response) => {
    const nodeId = req.params.nodeId;
    const node = assetGraph.getNode(assetGraph.getNodeIdByContentKey(nodeId));

    res.json(node);
  });

  app.get('/api/stats', (_req: Request, res: Response) => {
    const stats = getCacheStats(cache);
    res.json(stats);
  });

  app.get('/api/cache-keys/', (req: Request, res: Response) => {
    const sortBy = req.query.sortBy as string;
    const limit = req.query.limit != null ? Number(req.query.limit) : 100;
    const cursor = req.query.cursor as string | undefined;

    let resultKeys = keys;
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

  app.get('/api/cache-value/:key', (req: Request, res: Response) => {
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

  app.use(
    '/api/bundle-graph/related-bundles',
    (req: Request, res: Response) => {
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
    },
  );

  app.listen(3000, () => {
    // eslint-disable-next-line no-console
    console.log('Server is running on http://localhost:3000');
  });
}

main();
