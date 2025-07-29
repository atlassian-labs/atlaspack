import invariant from 'assert';
import nullthrows from 'nullthrows';
import {ContentGraph} from '@atlaspack/graph';
import type {NodeId} from '@atlaspack/graph';
import {setUnion, setIntersectStatic} from '@atlaspack/utils';
import type {Bundle, IdealBundleGraph} from './idealGraph';
import {memoize, clearCaches} from './memoize';

function getBundlesForBundleGroup(
  bundleGraph: IdealBundleGraph,
  bundleGroupId: NodeId,
): number {
  let count = 0;
  bundleGraph.traverse((nodeId) => {
    // @ts-expect-error TS2339
    if (bundleGraph.getNode(nodeId)?.bundleBehavior !== 'inline') {
      count++;
    }
  }, bundleGroupId);
  return count;
}

let getBundleOverlap = (
  sourceBundlesA: Set<NodeId>,
  sourceBundlesB: Set<NodeId>,
): number => {
  let allSourceBundles = setUnion(sourceBundlesA, sourceBundlesB);
  let sharedSourceBundles = setIntersectStatic(sourceBundlesA, sourceBundlesB);

  return sharedSourceBundles.size / allSourceBundles.size;
};

// Returns a decimal showing the proportion source bundles are common to
// both bundles versus the total number of source bundles.
function checkBundleThreshold(
  bundleA: MergeCandidate,
  bundleB: MergeCandidate,
  threshold: number,
): boolean {
  return (
    getBundleOverlap(
      bundleA.bundle.sourceBundles,
      bundleB.bundle.sourceBundles,
    ) >= threshold
  );
}

let checkSharedSourceBundles = memoize(
  (bundle: Bundle, importantAncestorBundles: Array<NodeId>): boolean => {
    return importantAncestorBundles.every((ancestorId) =>
      bundle.sourceBundles.has(ancestorId),
    );
  },
);

let hasSuitableBundleGroup = memoize(
  (
    bundleGraph: IdealBundleGraph,
    bundle: Bundle,
    minBundlesInGroup: number,
  ): boolean => {
    for (let sourceBundle of bundle.sourceBundles) {
      let bundlesInGroup = getBundlesForBundleGroup(bundleGraph, sourceBundle);

      if (bundlesInGroup >= minBundlesInGroup) {
        return true;
      }
    }
    return false;
  },
);

function validMerge(
  bundleGraph: IdealBundleGraph,
  config: MergeGroup,
  bundleA: MergeCandidate,
  bundleB: MergeCandidate,
): boolean {
  if (config.maxBundleSize != null) {
    if (
      bundleA.bundle.size > config.maxBundleSize ||
      bundleB.bundle.size > config.maxBundleSize
    ) {
      return false;
    }
  }

  if (config.overlapThreshold != null) {
    if (!checkBundleThreshold(bundleA, bundleB, config.overlapThreshold)) {
      return false;
    }
  }

  if (config.sourceBundles != null) {
    if (
      !checkSharedSourceBundles(bundleA.bundle, config.sourceBundles) ||
      !checkSharedSourceBundles(bundleB.bundle, config.sourceBundles)
    ) {
      return false;
    }
  }

  if (config.minBundlesInGroup != null) {
    if (
      !hasSuitableBundleGroup(
        bundleGraph,
        bundleA.bundle,
        config.minBundlesInGroup,
      ) ||
      !hasSuitableBundleGroup(
        bundleGraph,
        bundleB.bundle,
        config.minBundlesInGroup,
      )
    ) {
      return false;
    }
  }

  return true;
}

function getMergeClusters(
  graph: ContentGraph<NodeId, EdgeType>,
  candidates: Map<NodeId, EdgeType>,
): Array<Array<NodeId>> {
  let clusters: Array<Array<NodeId>> = [];

  for (let [candidate, edgeType] of candidates.entries()) {
    let cluster: Array<NodeId> = [];

    graph.traverse(
      (nodeId) => {
        cluster.push(nullthrows(graph.getNode(nodeId)));
        // Remove node from candidates as it has already been processed
        candidates.delete(nodeId);
      },
      candidate,
      edgeType,
    );
    clusters.push(cluster);
  }

  return clusters;
}

type MergeCandidate = {
  bundle: Bundle;
  id: NodeId;
  contentKey: string;
};
function getPossibleMergeCandidates(
  bundleGraph: IdealBundleGraph,
  bundles: Array<NodeId>,
): Array<[MergeCandidate, MergeCandidate]> {
  let mergeCandidates = bundles.map((bundleId) => {
    let bundle = bundleGraph.getNode(bundleId);
    invariant(bundle && bundle !== 'root', 'Bundle should exist');

    return {
      id: bundleId,
      bundle,
      contentKey: bundleId.toString(),
    };
  });

  const uniquePairs: Array<[MergeCandidate, MergeCandidate]> = [];

  for (let i = 0; i < mergeCandidates.length; i++) {
    for (let j = i + 1; j < mergeCandidates.length; j++) {
      let a = mergeCandidates[i];
      let b = mergeCandidates[j];

      // @ts-expect-error TS18048
      if (a.bundle.internalizedAssets.equals(b.bundle.internalizedAssets)) {
        uniquePairs.push([a, b]);
      }
    }
  }
  return uniquePairs;
}

export type MergeGroup = {
  overlapThreshold?: number;
  maxBundleSize?: number;
  sourceBundles?: Array<NodeId>;
  minBundlesInGroup?: number;
};
type EdgeType = number;

export function findMergeCandidates(
  bundleGraph: IdealBundleGraph,
  bundles: Array<NodeId>,
  config: Array<MergeGroup>,
): Array<Array<NodeId>> {
  let graph = new ContentGraph<NodeId, EdgeType>();
  let candidates = new Map<NodeId, EdgeType>();

  let allPossibleMergeCandidates = getPossibleMergeCandidates(
    bundleGraph,
    bundles,
  );

  // Build graph of clustered merge candidates
  for (let i = 0; i < config.length; i++) {
    // Ensure edge type coresponds to config index
    let edgeType = i + 1;

    for (let group of allPossibleMergeCandidates) {
      let candidateA = group[0];
      let candidateB = group[1];

      if (!validMerge(bundleGraph, config[i], candidateA, candidateB)) {
        continue;
      }

      let bundleNode = graph.addNodeByContentKeyIfNeeded(
        candidateA.contentKey,
        candidateA.id,
      );
      let otherBundleNode = graph.addNodeByContentKeyIfNeeded(
        candidateB.contentKey,
        candidateB.id,
      );

      // Add edge in both directions
      graph.addEdge(bundleNode, otherBundleNode, edgeType);
      graph.addEdge(otherBundleNode, bundleNode, edgeType);

      candidates.set(bundleNode, edgeType);
      candidates.set(otherBundleNode, edgeType);
    }

    // Remove bundles that have been allocated to a higher priority merge
    allPossibleMergeCandidates = allPossibleMergeCandidates.filter(
      (group) =>
        !graph.hasContentKey(group[0].contentKey) &&
        !graph.hasContentKey(group[1].contentKey),
    );
  }

  clearCaches();

  return getMergeClusters(graph, candidates);
}
