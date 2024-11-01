// @flow strict-local

import type {NodeId} from '../../../core/graph/src/types';
import type {BundleGraph, Bundle, Asset, Dependency} from '@atlaspack/types';
import type {MutableBundleGraph} from '@atlaspack/types';
import {ContentGraph} from '@atlaspack/graph';

import nullthrows from 'nullthrows';

export type DominatorBundlerInput<B: Bundle> = {|
  inputGraph: BundleGraph<B>,
  outputGraph: MutableBundleGraph,
  entries: {|entryAsset: Asset, entryDependency: Dependency|}[],
|};

export type DominatorBundlerOutput = {|
  bundleGraph: BundleGraph<Bundle>,
|};

export function dominatorBundler<B: Bundle>({
  inputGraph,
  outputGraph,
  entries,
}: DominatorBundlerInput<B>): DominatorBundlerOutput {
  throw new Error('Not implemented');
}

export type PackageNode = {|
  type: 'package',
  id: string,
  parentChunks: Set<number>,
|};

export type PackagedDominatorGraphNode = 'root' | Asset | PackageNode;

export type PackagedDominatorGraph = ContentGraph<PackagedDominatorGraphNode>;

/**
 * Start with the dominator graph. This is the immediate dominator tree, with a
 * root node at the top of every node.
 *
 * - Each node connected to a virtual root is a "chunk".
 * - We want to reduce the number of chunks by merging them together if they
 *   share the same parent chunks; this means those sub-trees are used together
 * - To do this we iterate over these chunks and build a map of parent chunks
 *   - This is done by looking at the parent assets of the chunk root asset
 *     and mapping them to parent chunks
 *   - We end-up with a relationship of "parent-ids": ["chunk", ...]
 * - For each group of chunks with the same set of parent chunks, we merge them
 *   under a new virtual node, called a package
 * - We repeat this process iteratively until no more merges are possible
 */
export function createPackages(
  bundleGraph: MutableBundleGraph,
  dominators: ContentGraph<Asset | 'root'>,
  onIteration?: (packages: PackagedDominatorGraph, label: string) => void,
): PackagedDominatorGraph {
  let changed = true;
  // $FlowFixMe
  const packages: PackagedDominatorGraph = dominators.clone();
  const nodesToRemove = [];

  const root = packages.getNodeIdByContentKey('root');
  let iterations = 0;
  while (changed) {
    changed = false;

    const chunks = getChunks(packages);
    // We don't need to keep running this on each iteration, since this changes
    // very little and we can track the changes and update this in-place.
    const chunksByAsset = getChunksByAsset(packages, chunks);
    const chunksByParent = getChunksByParent(
      chunks,
      packages,
      bundleGraph,
      chunksByAsset,
    );

    const merge = (parentChunk: number, chunksToMerge: number[]) => {
      for (const chunk of chunksToMerge) {
        if (parentChunk === chunk) {
          continue;
        }

        packages.removeEdge(root, chunk, 1, false);
        packages.addEdge(parentChunk, chunk);

        const chunkNode = packages.getNode(chunk);
        const isPackage =
          chunkNode != null &&
          chunkNode !== 'root' &&
          chunkNode.type === 'package';

        // Do work to flatten package tree
        if (isPackage) {
          const children = packages.getNodeIdsConnectedFrom(chunk);
          for (let child of children) {
            packages.addEdge(parentChunk, child);
          }
          nodesToRemove.push(chunk);
        }

        changed = true;
      }
    };

    for (const [key, {chunksToMerge, parentChunks}] of chunksByParent) {
      if (key === 'root') {
        continue;
      }

      // // If we're merging into a single parent then reparent
      if (parentChunks.size === 1) {
        const parentChunk = nullthrows(parentChunks.values().next().value);
        merge(parentChunk, chunksToMerge);
        continue;
      }

      const chunkRoot = packages.addNodeByContentKeyIfNeeded(`virtual:${key}`, {
        type: 'package',
        id: key,
        parentChunks,
      });
      packages.addEdge(root, chunkRoot);

      merge(chunkRoot, chunksToMerge);
    }

    onIteration?.(packages, String(iterations));
    iterations += 1;
  }

  // It is not ideal that we let the graph grow with virtual nodes while
  // building and only clean-up at the end. There'll be maximum `N*iterations`
  // nodes.
  for (let node of nodesToRemove) {
    packages.removeNode(node);
  }

  return packages;
}

/**
 * Build a Map of parent chunks to the chunks that share this parent.
 *
 * This will be used to merge the chunks that have the same parents.
 */
function getChunksByParent(
  chunks: NodeId[],
  packages: PackagedDominatorGraph,
  bundleGraph: MutableBundleGraph,
  chunksByAsset: Map<NodeId, NodeId>,
): Map<string, {|chunksToMerge: NodeId[], parentChunks: Set<NodeId>|}> {
  const chunksByParent = new Map();
  const makePackageKey = (parentChunks: Set<NodeId>): string =>
    parentChunks.size === 0
      ? 'root'
      : Array.from(parentChunks).sort().join(',');
  const addChunkToParent = (
    key: string,
    chunk: NodeId,
    parentChunks: Set<NodeId>,
  ) => {
    if (!chunksByParent.has(key)) {
      chunksByParent.set(key, {
        chunksToMerge: [],
        parentChunks,
      });
    }
    chunksByParent.get(key)?.chunksToMerge.push(chunk);
  };

  for (let chunk of chunks) {
    const chunkNode = packages.getNode(chunk);
    if (chunkNode == null || chunkNode === 'root') continue; // can't happen

    const parentChunks =
      chunkNode.type === 'package'
        ? // $FlowFixMe the types are messed-up right now because `Asset` has a `type` field
          getPackageParentChunks(packages, chunksByAsset, chunkNode)
        : getAssetParentChunks(bundleGraph, packages, chunksByAsset, chunkNode);

    const key = makePackageKey(parentChunks);
    addChunkToParent(key, chunk, parentChunks);
  }

  return chunksByParent;
}

/**
 * Given a package node, which is at the root of a chunk, return the nodes above
 * it. This is already in the node itself. However, after the first iteration,
 * the parent chunks will be stale, so we need to look them up in the
 * `chunksByAsset` map to effectively walk up the tree from a parent to the
 * most up-to-date root chunk.
 */
export function getPackageParentChunks(
  packages: PackagedDominatorGraph,
  chunksByAsset: Map<NodeId, NodeId>,
  chunkNode: PackageNode,
): Set<NodeId> {
  return new Set(
    Array.from(chunkNode.parentChunks).map((c) => chunksByAsset.get(c) ?? c),
  );
}

/**
 * Given an Asset which is at the root of a chunk, figure out which other chunks
 * depend on it.
 *
 * This will be used to merge chunks that have the same parents.
 */
export function getAssetParentChunks(
  bundleGraph: MutableBundleGraph,
  packages: PackagedDominatorGraph,
  chunksByAsset: Map<NodeId, NodeId>,
  chunkNode: Asset,
): Set<NodeId> {
  const parentDependencies = bundleGraph.getIncomingDependencies(chunkNode);
  const parentAssets = parentDependencies.map((dep) => {
    return bundleGraph.getAssetWithDependency(dep);
  });
  const parentChunks = new Set();

  parentAssets.forEach((asset) => {
    if (!asset) return;
    const assetNodeId = packages.getNodeIdByContentKey(asset.id);
    const chunk = chunksByAsset.get(assetNodeId);
    // chunk is null if parent is the root
    if (chunk != null) {
      parentChunks.add(chunk);
    }
  });

  return parentChunks;
}

/**
 * Nodes connected to the root node are considered chunks.
 */
function getChunks(dominatorTree: PackagedDominatorGraph): NodeId[] {
  return dominatorTree.getNodeIdsConnectedFrom(
    dominatorTree.getNodeIdByContentKey('root'),
  );
}

/**
 * Build a map that tells us which chunk a given asset belongs to.
 *
 * This is done by traversing down the dominator tree from each chunk root.
 *
 * This is a O(roots) operation.
 */
function getChunksByAsset(
  packages: PackagedDominatorGraph,
  chunks: NodeId[],
): Map<NodeId, NodeId> {
  const chunksByAsset = new Map();
  for (let chunk of chunks) {
    packages.traverse((node) => {
      if (node === packages.rootNodeId) return;
      chunksByAsset.set(node, chunk);
    }, chunk);
  }
  return chunksByAsset;
}
