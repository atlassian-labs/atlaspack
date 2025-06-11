import type BundleGraph from '@atlaspack/core/lib/BundleGraph.js';
import {requestTypes} from '@atlaspack/core/lib/RequestTracker.js';
import type RequestTracker from '@atlaspack/core/lib/RequestTracker.js';
import type {RequestGraphNode} from '@atlaspack/core/lib/RequestTracker.js';
import type {Node} from '@atlaspack/core/lib/types.js';
import fs from 'fs';
import path from 'path';

export type AssetTreeNode = {
  children: Record<string, AssetTreeNode>;
  size: number;
  path: string;
};

export interface TreemapBundle {
  id: string;
  displayName: string;
  bundle: any;
  size: number;
  filePath: string;
  assetTree: AssetTreeNode;
}

export interface Treemap {
  bundles: Array<TreemapBundle>;
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
    writeBundleRequests.map((request) => [
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
export function buildTreemapBundle(
  writeBundleRequestsByBundleId: Map<string, RequestGraphNode>,
  node: Node,
  projectRoot: string,
  bundleGraph: BundleGraph,
): TreemapBundle {
  const writeBundleRequest = writeBundleRequestsByBundleId.get(node.id);
  const filePath = writeBundleRequest?.result?.filePath
    ? path.join(projectRoot, writeBundleRequest.result.filePath)
    : null;
  const size = filePath ? fs.statSync(filePath).size : 0;

  // this is the directory structure
  const assetTree: AssetTreeNode = {path: '', children: {}, size: 0};

  const assets: any[] = [];
  // @ts-expect-error
  bundleGraph.traverseAssets(node.value, (asset) => {
    assets.push(asset);
  });

  for (const asset of assets) {
    const filePath = asset.filePath;
    const parts = filePath.split('/');
    const assetSize = asset.stats.size;
    let current = assetTree;

    for (let part of parts) {
      current.size += assetSize;
      current.children[part] = current.children[part] ?? {
        path: parts.slice(0, parts.indexOf(part) + 1).join('/'),
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

export function buildTreemap(
  projectRoot: string,
  bundleGraph: BundleGraph,
  requestTracker: RequestTracker,
): Treemap {
  const treemap: Treemap = {
    bundles: [],
    totalSize: 0,
  };

  const writeBundleRequestsByBundleId =
    getWriteBundleRequestsByBundleId(requestTracker);

  bundleGraph._graph.nodes.forEach((node: Node) => {
    if (node.type === 'bundle') {
      const treemapBundle = buildTreemapBundle(
        writeBundleRequestsByBundleId,
        node,
        projectRoot,
        bundleGraph,
      );

      treemap.bundles.push(treemapBundle);
      treemap.totalSize += treemapBundle.size;
    }
  });

  return treemap;
}
