/* eslint-disable monorepo/no-internal-import */
// @ts-expect-error TS2749
import type BundleGraph from '@atlaspack/core/lib/BundleGraph.js';
// @ts-expect-error TS2749
import {requestTypes} from '@atlaspack/core/lib/RequestTracker.js';
// @ts-expect-error TS2749
import type RequestTracker from '@atlaspack/core/lib/RequestTracker.js';
// @ts-expect-error TS2749
import type {RequestGraphNode} from '@atlaspack/core/lib/RequestTracker.js';
// @ts-expect-error TS2749
import type {Node} from '@atlaspack/core/lib/types.js';
import fs from 'fs';
import path from 'path';
import {logger} from '../config/logger';

/**
 * An asset tree node.
 *
 * This is the file-tree structure of the bundle. Each file and directory is a node in the
 * tree.
 *
 * Directories' sizes are the sum of their children's sizes.
 *
 * Sizes are read from disk, not from the bundle itself.
 */
export type AssetTreeNode = {
  /**
   * ID of this file.
   */
  id: string;
  /**
   * Children of this node (empty is this is a file).
   *
   * This is the next path component in the file-path leading to each child.
   */
  children: Record<string, AssetTreeNode>;
  /**
   * Size of this node in bytes.
   */
  size: number;
  /**
   * The full file-path of this node.
   */
  path: string;
};

/**
 * A bundle in the tree-map.
 */
export interface TreemapBundle {
  /**
   * The bundle `id` for tree-map purposes.
   */
  id: string;
  /**
   * A display name for the UI
   */
  displayName: string;
  /**
   * Raw bundle node from the bundle graph.
   */
  bundle: any;
  /**
   * Size of the bundle in bytes, as read from disk.
   */
  size: number;
  /**
   * The file path of the bundle on disk.
   */
  filePath: string;
  /**
   * The file-tree for the bundle.
   */
  assetTree: AssetTreeNode;
}

/**
 * The tree-map model.
 */
export interface Treemap {
  /**
   * Each bundle in the application build.
   */
  bundles: Array<TreemapBundle>;
  /**
   * Total size of all bundles in bytes.
   */
  totalSize: number;
}

/**
 * In order to find the sizes of bundles, we look-up write bundle requests for each bundle
 * in this build, then read the sizes of each file from disk.
 */
export function getWriteBundleRequestsByBundleId(
  requestTracker: RequestTracker,
): Map<string, RequestGraphNode> {
  const writeBundleRequests = requestTracker.graph.nodes.filter(
    (requestNode: RequestGraphNode) =>
      requestNode &&
      requestNode.type === 1 &&
      requestNode.requestType === requestTypes.write_bundle_request,
  );

  return new Map(
    writeBundleRequests.map((request: RequestGraphNode) => [
      request.result?.bundleId ?? '',
      request,
    ]),
  );
}

/**
 * Builds a tree-map model for a bundle graph.
 *
 * The tree-map is a tree structure starting at the bundle and having layers for
 * each asset sub-directory in its asset tree.
 *
 * Asset sizes are calculated from the asset size stats, which should represent
 * sizes post-transformation, but before minification.
 *
 * Bundle sizes are read from the bundle files on disk.
 *
 * The sub-directories are sized to the size of their children.
 */
export function buildTreemapBundle({
  writeBundleRequestsByBundleId,
  node,
  projectRoot,
  repositoryRoot,
  bundleGraph,
}: {
  writeBundleRequestsByBundleId: Map<string, RequestGraphNode>;
  node: Node;
  projectRoot: string;
  repositoryRoot: string;
  bundleGraph: BundleGraph;
}): TreemapBundle {
  const writeBundleRequest = writeBundleRequestsByBundleId.get(node.id);
  const {size, filePath} = findBundleInfo({
    writeBundleRequest,
    projectRoot,
    repositoryRoot,
    node,
  });

  // this is the directory structure
  const assetTree: AssetTreeNode = {
    id: `${node.id}::/`,
    path: '',
    children: {},
    size: 0,
  };

  const assets: any[] = [];
  bundleGraph.traverseAssets(node.value, (asset: any) => {
    assets.push(asset);
  });

  for (const asset of assets) {
    const filePath = path.isAbsolute(asset.filePath)
      ? path.relative(repositoryRoot, asset.filePath)
      : asset.filePath;
    const parts = filePath.split('/');
    const assetSize = asset.stats.size;
    let current = assetTree;

    let currentSubpath = '';
    for (let part of parts) {
      currentSubpath += '/' + part;
      current.size += assetSize;
      current.children[part] = current.children[part] ?? {
        id: `${node.id}::${currentSubpath}`,
        path: currentSubpath,
        children: {},
        size: 0,
      };
      current = current.children[part];
    }
    current.size += assetSize;
  }

  const treemapBundle = {
    id: node.id,
    displayName: node.value.displayName,
    bundle: node,
    size,
    filePath: filePath ?? '',
    assetTree,
  };
  return treemapBundle;
}

/**
 * Finds the size and file path of a bundle.
 *
 * If the file path is not absolute, it is made absolute by joining it with the project root.
 *
 * If the file path is not found, the size is 0.
 *
 * If the file path is found but the file cannot be read, an error is logged and the size is 0.
 */
export function findBundleInfo({
  writeBundleRequest,
  projectRoot,
  repositoryRoot,
  node,
}: {
  writeBundleRequest: RequestGraphNode | undefined;
  projectRoot: string;
  repositoryRoot: string;
  node: Node;
}): {size: number; filePath: string | null} {
  let filePath = writeBundleRequest?.result?.filePath
    ? path.isAbsolute(writeBundleRequest?.result?.filePath)
      ? writeBundleRequest.result.filePath
      : path.join(projectRoot, writeBundleRequest.result.filePath)
    : null;
  let size = 0;
  try {
    size = filePath ? fs.statSync(filePath).size : 0;
  } catch (e) {
    logger.error(
      {filePath, nodeId: node.id, error: e},
      `Error getting size of bundle at ${filePath}`,
    );
  }

  filePath = filePath
    ? path.isAbsolute(filePath)
      ? path.relative(repositoryRoot, filePath)
      : filePath
    : null;

  return {size, filePath};
}

export function buildTreemap({
  projectRoot,
  repositoryRoot,
  bundleGraph,
  requestTracker,
}: {
  projectRoot: string;
  repositoryRoot: string;
  bundleGraph: BundleGraph;
  requestTracker: RequestTracker;
}): Treemap {
  const treemap: Treemap = {
    bundles: [],
    totalSize: 0,
  };

  const writeBundleRequestsByBundleId =
    getWriteBundleRequestsByBundleId(requestTracker);

  bundleGraph._graph.nodes.forEach((node: Node) => {
    if (node.type === 'bundle') {
      const treemapBundle = buildTreemapBundle({
        writeBundleRequestsByBundleId,
        node,
        projectRoot,
        repositoryRoot,
        bundleGraph,
      });

      treemap.bundles.push(treemapBundle);
      treemap.totalSize += treemapBundle.size;
    }
  });

  return treemap;
}
