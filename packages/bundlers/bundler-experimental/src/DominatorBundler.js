// @flow strict-local

import type {BundleGraph, Bundle, Asset, Dependency} from '@atlaspack/types';
import type {MutableBundleGraph} from '@atlaspack/types';
import {bundleGraphToRootedGraph} from './DominatorBundler/bundleGraphToRootedGraph';
import {ContentGraph, type NodeId} from '@atlaspack/graph';
import type {StronglyConnectedComponentNode} from './DominatorBundler/oneCycleBreaker';

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
  parentChunks: Set<string>,
|};

export type PackagedDominatorGraphNode = 'root' | Asset | PackageNode;

export type PackagedDominatorGraph = ContentGraph<PackagedDominatorGraphNode>;

export type PackagingInputGraph = ContentGraph<
  Asset | StronglyConnectedComponentNode<Asset | 'root'> | 'root',
>;

/**
 * Start with the dominator graph. This is the immediate dominator tree, with a
 * root node at the top of every node.
 *
 * - Each node connected to a virtual root is a "chunk".
 * - We want to reduce the number of chunks by merging them together if they
 *   share the same parent ENTRIES; this means those sub-trees are used together
 * - To do this we iterate over these chunks and build a map of parent ENTRIES
 * - For each group of chunks with the same set of parent chunks, we merge them
 *   under a new virtual node, called a package
 */
export function createPackages(
  bundleGraph: MutableBundleGraph,
  dominators: PackagingInputGraph,
): PackagedDominatorGraph {
  // $FlowFixMe
  const packages: PackagedDominatorGraph = dominators.clone();
  const nodesToRemove = [];

  const rootedGraph = bundleGraphToRootedGraph(bundleGraph);
  const root = packages.getNodeIdByContentKey('root');
  const chunks = getChunks(packages);
  const chunksByAsset = getChunkEntryPoints(rootedGraph, dominators);
  const chunksByParent = getChunksByParentEntryPoint(
    chunks,
    packages,
    bundleGraph,
    chunksByAsset,
  );

  const merge = (parentChunkAssetId: string, chunksToMerge: number[]) => {
    for (const chunk of chunksToMerge) {
      const parentChunk = packages.getNodeIdByContentKey(parentChunkAssetId);
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

    merge(`virtual:${key}`, chunksToMerge);
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
function getChunksByParentEntryPoint(
  chunks: NodeId[],
  packages: PackagedDominatorGraph,
  bundleGraph: MutableBundleGraph,
  entryPointsByChunk: Map<string, Set<string>>,
): Map<string, {|chunksToMerge: NodeId[], parentChunks: Set<string>|}> {
  const chunksByParent = new Map();
  const makePackageKey = (parentChunks: Set<string>): string =>
    parentChunks.size === 0
      ? 'root'
      : Array.from(parentChunks).sort().join(',');
  const addChunkToParent = (
    key: string,
    chunk: NodeId,
    parentChunks: Set<string>,
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

    const parentChunks = entryPointsByChunk.get(chunkNode.id) ?? new Set();
    const key = makePackageKey(parentChunks);
    addChunkToParent(key, chunk, parentChunks);
  }

  return chunksByParent;
}

export function getChunkEntryPoints(
  rootedGraph: ContentGraph<'root' | Asset>,
  dominators: PackagingInputGraph,
): Map<string, Set<string>> {
  const chunks = getChunks(dominators).map((id) => {
    const node = dominators.getNode(id);
    if (node == null || node === 'root') {
      throw new Error('Invariant violation');
    }

    return node.id;
  });

  const chunkSet = new Set(chunks);
  const result = new Map();
  const addChunk = (chunk: string, entryPoint: string) => {
    if (!result.has(chunk)) {
      result.set(chunk, new Set());
    }
    result.get(chunk)?.add(entryPoint);
  };

  const root = rootedGraph.getNodeIdByContentKey('root');
  const entryPoints = rootedGraph.getNodeIdsConnectedFrom(root);
  const entryPointsSet = new Set(entryPoints);

  for (let entryPointId of entryPoints) {
    const entryPoint = getAssetNode(rootedGraph, entryPointId);

    rootedGraph.traverse((nodeId) => {
      if (
        nodeId === root ||
        entryPointId === nodeId ||
        entryPointsSet.has(nodeId)
      ) {
        return;
      }

      const node = getAssetNode(rootedGraph, nodeId);
      const isChunk = chunkSet.has(node.id);
      if (isChunk) {
        addChunk(node.id, entryPoint.id);
      }
    }, entryPointId);
  }

  return result;
}

/**
 * Nodes connected to the root node are considered chunks.
 */
function getChunks<T>(dominatorTree: ContentGraph<T>): NodeId[] {
  return dominatorTree.getNodeIdsConnectedFrom(
    dominatorTree.getNodeIdByContentKey('root'),
  );
}

export function getAssetNodeByKey(
  graph:
    | ContentGraph<'root' | Asset>
    | ContentGraph<
        'root' | StronglyConnectedComponentNode<'root' | Asset> | Asset,
      >,
  id: string,
): Asset {
  const node = graph.getNodeByContentKey(id);
  if (
    node == null ||
    node === 'root' ||
    node.type === 'StronglyConnectedComponent'
  ) {
    throw new Error('Invariant violation');
  }
  return node;
}

export function getAssetNode(
  graph:
    | ContentGraph<'root' | Asset>
    | ContentGraph<
        'root' | StronglyConnectedComponentNode<'root' | Asset> | Asset,
      >,
  id: NodeId,
): Asset {
  const node = graph.getNode(id);
  if (
    node == null ||
    node === 'root' ||
    node.type === 'StronglyConnectedComponent'
  ) {
    throw new Error('Invariant violation');
  }
  return node;
}
