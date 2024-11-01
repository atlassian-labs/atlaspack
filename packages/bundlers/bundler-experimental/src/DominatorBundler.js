// @flow strict-local

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
  throw new Error('');
}

export function bundleGraphToRootedGraph(
  bundleGraph: MutableBundleGraph,
): ContentGraph<'root' | Asset> {
  const graph = new ContentGraph();

  const rootNodeId = graph.addNodeByContentKey('root', 'root');
  graph.setRootNodeId(rootNodeId);

  const postOrder = [];
  bundleGraph.traverse({
    exit: (node) => {
      if (node.type === 'asset') {
        postOrder.push(node.value.id);
      }
    },
  });
  const reversedPostOrder = postOrder.slice().reverse();

  for (let assetId of reversedPostOrder) {
    const childAsset = bundleGraph.getAssetById(assetId);
    const assetNodeId = graph.addNodeByContentKey(assetId, childAsset);

    for (let dependency of bundleGraph.getIncomingDependencies(childAsset)) {
      if (dependency.isEntry) {
        graph.addEdge(rootNodeId, assetNodeId);
      } else {
        const parentAsset = bundleGraph.getAssetWithDependency(dependency);
        if (!parentAsset) {
          throw new Error('Non entry dependency had no asset');
        }
        graph.addEdge(graph.getNodeIdByContentKey(parentAsset.id), assetNodeId);
      }
    }
  }

  return graph;
}

/**
 * For all assets, build the dominance relationship of the asset with other
 * assets.
 *
 * This implements "A simple, fast dominance algorithm" - https://www.cs.tufts.edu/comp/150FP/archive/keith-cooper/dom14.pdf
 *
 * The return value contains the **immediate** dominator tree, which is
 * different to the dominator tree.
 */
export function findAssetDominators(
  bundleGraph: MutableBundleGraph,
): ContentGraph<'root' | Asset> {
  // Build a simpler graph with a root at the top
  const graph = bundleGraphToRootedGraph(bundleGraph);

  const postOrder = [];
  graph.traverse({
    exit: (node) => {
      postOrder.push(node);
    },
  });
  const reversedPostOrder = postOrder.slice().reverse();

  const dominators = Array(graph.nodes.length).fill(null);
  // we know there's a root node
  dominators[graph.rootNodeId ?? 0] = graph.rootNodeId;

  let changed = true;

  while (changed) {
    changed = false;

    for (let node of reversedPostOrder) {
      if (node === graph.rootNodeId) continue;

      const predecessors = graph.getNodeIdsConnectedTo(node);

      let newImmediateDominator = predecessors[0];
      for (let i = 1; i < predecessors.length; i++) {
        const predecessor = predecessors[i];
        if (dominators[predecessor] == null) {
          continue;
        }

        newImmediateDominator = intersect(
          postOrder,
          dominators,
          predecessor,
          newImmediateDominator,
        );
      }

      if (dominators[node] !== newImmediateDominator) {
        dominators[node] = newImmediateDominator;
        changed = true;
      }
    }
  }

  const dominatorTree = new ContentGraph();
  const rootId = dominatorTree.addNodeByContentKey('root', 'root');
  dominatorTree.setRootNodeId(rootId);

  bundleGraph.traverse((node) => {
    if (node.type === 'asset') {
      dominatorTree.addNodeByContentKey(node.value.id, node.value);
    }
  });

  for (let nodeId = 0; nodeId < dominators.length; nodeId++) {
    const asset = graph.getNode(nodeId);
    if (asset === 'root' || asset == null) {
      continue;
    }

    const assetDominatorTreeNodeId = dominatorTree.getNodeIdByContentKey(
      asset.id,
    );

    const immediateDominator = dominators[nodeId];
    if (immediateDominator == null) continue;
    const immediateDominatorNode = graph.getNode(immediateDominator);
    if (immediateDominatorNode == null) continue;

    if (immediateDominatorNode === 'root') {
      dominatorTree.addEdge(rootId, assetDominatorTreeNodeId);
      continue;
    }

    const immediateDominatedId = dominatorTree.getNodeIdByContentKey(
      immediateDominatorNode.id,
    );
    dominatorTree.addEdge(immediateDominatedId, assetDominatorTreeNodeId);
  }

  return dominatorTree;
}

function intersect(
  postOrder: number[],
  dominators: (number | null)[],
  predecessor: number,
  newImmediateDominator: number,
) {
  let n1: number = predecessor;
  let n2: number = newImmediateDominator;
  while (n1 !== n2) {
    while (postOrder.indexOf(n1) < postOrder.indexOf(n2)) {
      n1 = Number(dominators[n1]);
    }
    while (postOrder.indexOf(n2) < postOrder.indexOf(n1)) {
      n2 = Number(dominators[n2]);
    }
  }
  return n1;
}

export type PackagedDominatorGraph = ContentGraph<
  'root' | Asset | {|type: 'package', id: string, parentChunks: Set<number>|},
>;

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

  let iterations = 0;
  while (changed) {
    console.log('RUNNING ITERATION');
    changed = false;

    const chunks = packages.getNodeIdsConnectedFrom(
      packages.getNodeIdByContentKey('root'),
    );
    console.log(chunks);
    const chunksByAsset = new Map();
    for (let chunk of chunks) {
      console.log('traverse', packages.getNode(chunk));
      packages.traverse((node) => {
        if (node === packages.rootNodeId) return;
        chunksByAsset.set(node, chunk);
        console.log(' ==> ', packages.getNode(node));
      }, chunk);
    }

    const chunksByParent = new Map();
    for (let chunk of chunks) {
      const chunkNode = packages.getNode(chunk);
      if (chunkNode == null || chunkNode === 'root') continue; // can't happen

      const getParentChunks = (chunkNode: Asset) => {
        const parentDependencies =
          bundleGraph.getIncomingDependencies(chunkNode);
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
      };

      const parentChunks =
        chunkNode.type === 'package'
          ? new Set(
              // $FlowFixMe the types are messed-up right now
              Array.from(chunkNode.parentChunks).map(
                (c) => chunksByAsset.get(c) ?? c,
              ),
            )
          : getParentChunks(chunkNode);

      console.log('chunkNode', chunkNode);
      console.log(
        'parentChunks',
        Array.from(parentChunks).map((c) => packages.getNode(c)),
      );

      const key =
        parentChunks.size === 0
          ? 'root'
          : Array.from(parentChunks).sort().join(',');
      if (!chunksByParent.has(key)) {
        chunksByParent.set(key, {
          chunksToMerge: [],
          parentChunks,
        });
      }
      chunksByParent.get(key)?.chunksToMerge.push(chunk);
    }

    console.log(chunksByParent);

    const merge = (parentChunk: number, chunksToMerge: number[]) => {
      for (const chunk of chunksToMerge) {
        if (parentChunk === chunk) {
          continue;
        }

        packages.removeEdge(
          packages.getNodeIdByContentKey('root'),
          chunk,
          1,
          false,
        );

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
      packages.addEdge(packages.getNodeIdByContentKey('root'), chunkRoot);

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
  onIteration?.(packages, 'clean-up');

  return packages;
}

export function isSetEqual(a: Set<Asset>, b: Set<Asset>): boolean {
  if (a.size !== b.size) {
    return false;
  }

  for (let item of a) {
    if (!b.has(item)) {
      return false;
    }
  }

  return true;
}
