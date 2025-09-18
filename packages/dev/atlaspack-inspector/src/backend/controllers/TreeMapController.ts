/* eslint-disable import/no-extraneous-dependencies */
/* eslint-disable monorepo/no-internal-import */
import {Router} from 'express';
import path from 'path';
import {
  getBundleGraph,
  getTreemap,
} from '../config/middleware/cacheDataMiddleware';
// @ts-expect-error TS2749
import {Node} from '@atlaspack/core/lib/types.js';
import {ALL_EDGE_TYPES} from '@atlaspack/graph';
import {SourceCodeURL} from '../services/findSourceCodeUrl';
import {logger} from '../config/logger';

export interface MakeTreemapControllerParams {
  sourceCodeURL: SourceCodeURL | null;
  projectRoot: string;
  repositoryRoot: string;
}

export function makeTreemapController({
  sourceCodeURL,
  projectRoot,
  repositoryRoot,
}: MakeTreemapControllerParams): Router {
  const router = Router();

  router.get('/api/treemap/reasons', (req, res) => {
    const bundleGraph = getBundleGraph(res);

    const queryPath = req.query.path as string;
    const bundle = req.query.bundle as string;

    const bundleNode = bundleGraph._graph.getNode(
      bundleGraph._graph.getNodeIdByContentKey(bundle),
    );

    const relevantPaths: string[][] = [];
    let tooManyPaths = false;
    bundleGraph.traverseBundle(bundleNode.value, {
      enter(
        node: Node,
        context: string[],
        actions: {skipChildren: () => void; stop: () => void},
      ) {
        if (context == null) {
          context = [];
        }

        if (node.type === 'asset') {
          context.push(path.relative(repositoryRoot, node.value.filePath));
        }

        if (node.type === 'dependency') {
          try {
            const childNodeIds = bundleGraph._graph.getNodeIdsConnectedFrom(
              bundleGraph._graph.getNodeIdByContentKey(node.id),
              ALL_EDGE_TYPES,
            );
            let isParent = false;
            for (const childNodeId of childNodeIds) {
              const childNode = bundleGraph._graph.getNode(childNodeId);
              if (
                childNode.type === 'asset' &&
                path
                  .relative(repositoryRoot, childNode.value.filePath)
                  .startsWith(queryPath)
              ) {
                actions.skipChildren();
                isParent = true;
              }
            }

            // For some reason we visit all nodes from the bundle, so we need to filter out
            // stuff that is directly connected to the bundle node, since that's not useful
            // information.
            // e.g.: On the cases where the file is included directly to the bundle either due
            // to manual bundling or entry dependencies, the user probably already knows about
            // it.
            if (isParent && context.length > 1) {
              relevantPaths.push(context.slice());

              if (relevantPaths.length > 50) {
                tooManyPaths = true;
                actions.stop();
              }
            }
          } catch (err) {
            logger.error(err);
          }
        }

        return context;
      },
      exit(node: Node, context: string[] = []) {
        if (node.type === 'asset') {
          context.pop();
        }
        return context;
      },
    });

    res.json({
      tooManyPaths,
      relevantPaths,
      sourceCodeURL,
      projectRoot,
      repositoryRoot,
    });
  });

  router.get('/api/treemap', (req, res) => {
    const treemap = getTreemap(res);

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
          id: `${bundle.id}::/`,
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

  return router;
}
