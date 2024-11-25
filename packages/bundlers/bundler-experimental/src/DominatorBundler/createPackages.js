// @flow strict-local

import invariant from 'assert';
import type {Asset} from '@atlaspack/types';
import type {MutableBundleGraph} from '@atlaspack/types';
import {ContentGraph, type NodeId} from '@atlaspack/graph';

import {bundleGraphToRootedGraph} from './bundleGraphToRootedGraph';
import type {
  AcyclicGraph,
  StronglyConnectedComponentNode,
} from './oneCycleBreaker';

import {EdgeContentGraph} from './EdgeContentGraph';
import nullthrows from 'nullthrows';
import type {
  AssetNode,
  SimpleAssetGraph,
  SimpleAssetGraphEdge,
  SimpleAssetGraphEdgeWeight,
} from './bundleGraphToRootedGraph';

export type PackageNode = {|
  type: 'package',
  id: string,
  parentChunks: Set<string>,
  entryPointAssets: Set<AssetNode>,
|};

export type PackagedDominatorGraphNode =
  | 'root'
  | AssetNode
  | PackageNode
  | StronglyConnectedComponentNode<AssetNode>;

export type PackagedDominatorGraph = EdgeContentGraph<
  PackagedDominatorGraphNode,
  SimpleAssetGraphEdgeWeight,
>;

export type PackagingInputGraph = AcyclicGraph<
  'root' | AssetNode,
  SimpleAssetGraphEdgeWeight,
>;

export function getPackageNodes(packages: PackagedDominatorGraph): NodeId[] {
  return packages.getNodeIdsConnectedFrom(
    packages.getNodeIdByContentKey('root'),
  );
}

const defaultMakePackageKey = (parentChunks: Set<string>): string =>
  parentChunks.size === 0 ? 'root' : Array.from(parentChunks).sort().join(',');

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
  makePackageKey: (parentChunks: Set<string>) => string = defaultMakePackageKey,
): PackagedDominatorGraph {
  // $FlowFixMe
  const packages: PackagedDominatorGraph = dominators.clone();

  const rootedGraph = bundleGraphToRootedGraph(bundleGraph);
  const root = packages.getNodeIdByContentKey('root');
  const chunks = getChunks(packages);
  const chunksByAsset = getChunkEntryPoints(rootedGraph, dominators);
  const chunksByParent = getChunksByParentEntryPoint(
    chunks,
    packages,
    bundleGraph,
    chunksByAsset,
    makePackageKey,
  );

  const nodesToRemove = new Set();
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
        nodesToRemove.add(chunk);
      }
    }
  };

  for (const [
    key,
    {chunksToMerge, parentChunks, entryPointAssets},
  ] of chunksByParent) {
    if (key === 'root') {
      continue;
    }

    // If we're merging into a single parent then re-parent
    if (parentChunks.size === 1) {
      const parentChunk = nullthrows(parentChunks.values().next().value);
      merge(parentChunk, chunksToMerge);
      continue;
    }

    console.log('creating package', 'for chunks', chunksToMerge);
    const chunkRoot = packages.addNodeByContentKeyIfNeeded(`package:${key}`, {
      type: 'package',
      id: `package:${key}`,
      parentChunks,
      entryPointAssets,
    });
    packages.addEdge(root, chunkRoot);

    merge(`package:${key}`, chunksToMerge);
  }

  for (let node of nodesToRemove) {
    packages.removeNode(node, false);
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
  entryPointsByChunk: Map<
    string,
    {|entryPointChunkIds: Set<string>, entryPointAssets: Set<AssetNode>|},
  >,
  makePackageKey: (parentChunks: Set<string>) => string,
): Map<
  string,
  {|
    chunksToMerge: NodeId[],
    parentChunks: Set<string>,
    entryPointAssets: Set<AssetNode>,
  |},
> {
  const chunksByParent = new Map();
  const addChunkToParent = (
    key: string,
    chunk: NodeId,
    parentChunks: Set<string>,
    entryPointAssets: Set<AssetNode>,
  ) => {
    if (!chunksByParent.has(key)) {
      chunksByParent.set(key, {
        chunksToMerge: [],
        parentChunks,
        entryPointAssets,
      });
    }
    chunksByParent.get(key)?.chunksToMerge.push(chunk);
  };

  for (let chunk of chunks) {
    const chunkNode = packages.getNode(chunk);
    if (chunkNode == null || chunkNode === 'root') continue; // can't happen

    const parentChunks = entryPointsByChunk.get(chunkNode.id) ?? {
      entryPointChunkIds: new Set(),
      entryPointAssets: new Set(),
    };

    const key = makePackageKey(parentChunks.entryPointChunkIds);
    addChunkToParent(
      key,
      chunk,
      parentChunks.entryPointChunkIds,
      parentChunks.entryPointAssets,
    );
  }

  return chunksByParent;
}

export function getChunkEntryPoints(
  rootedGraph: SimpleAssetGraph,
  dominators: PackagingInputGraph,
): Map<
  string,
  {|entryPointChunkIds: Set<string>, entryPointAssets: Set<AssetNode>|},
> {
  const chunks = getChunks(dominators).map((id) => {
    const node = dominators.getNode(id);
    if (node == null || node === 'root') {
      throw new Error('Invariant violation');
    }

    return node.id;
  });

  const chunkSet = new Set(chunks);
  const result = new Map();
  const addChunk = (
    chunk: string,
    entryPoint: string,
    entryPointAsset: AssetNode,
  ) => {
    if (!result.has(chunk)) {
      result.set(chunk, {
        entryPointChunkIds: new Set(),
        entryPointAssets: new Set(),
      });
    }

    const existing = result.get(chunk);
    invariant(existing != null);
    existing.entryPointChunkIds.add(entryPoint);
    existing.entryPointAssets.add(entryPointAsset);
  };

  const root = rootedGraph.getNodeIdByContentKey('root');
  const entryPoints = rootedGraph.getNodeIdsConnectedFrom(root);
  const entryPointsSet = new Set(entryPoints);

  for (let entryPointId of entryPoints) {
    const entryPointNode = rootedGraph.getNode(entryPointId);
    invariant(
      entryPointNode != null &&
        entryPointNode !== 'root' &&
        entryPointNode.type === 'asset',
    );
    const entryPoint = entryPointNode.asset;

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
        addChunk(node.id, entryPoint.id, entryPointNode);
      }
    }, entryPointId);
  }

  return result;
}

/**
 * Nodes connected to the root node are considered chunks.
 */
function getChunks<T>(dominatorTree: ContentGraph<T, number>): NodeId[] {
  return dominatorTree.getNodeIdsConnectedFrom(
    dominatorTree.getNodeIdByContentKey('root'),
  );
}

export function getAssetNodeByKey(
  graph:
    | SimpleAssetGraph
    | AcyclicGraph<AssetNode | 'root', SimpleAssetGraphEdgeWeight>,
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
  return node.asset;
}

export function getAssetNode(
  graph:
    | SimpleAssetGraph
    | AcyclicGraph<AssetNode | 'root', SimpleAssetGraphEdgeWeight>,
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
  return node.asset;
}
