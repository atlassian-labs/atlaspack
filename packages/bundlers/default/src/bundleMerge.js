// @flow strict-local

import invariant from 'assert';
import type {NodeId} from '@atlaspack/graph';
import type {Bundle, IdealBundleGraph} from './idealGraph';
import {ContentGraph} from '@atlaspack/graph';

// Returns a decimal showing the proportion source bundles are common to
// both bundles versus the total number of source bundles.
function scoreBundleMerge(bundleA: Bundle, bundleB: Bundle): number {
  let sharedSourceBundles = 0;
  let allSourceBundles = new Set([
    ...bundleA.sourceBundles,
    ...bundleB.sourceBundles,
  ]);

  for (let bundle of bundleB.sourceBundles) {
    if (bundleA.sourceBundles.has(bundle)) {
      sharedSourceBundles++;
    }
  }

  let overlapScore = sharedSourceBundles / allSourceBundles.size;
  let sizeScore = 1 / Math.log10(Math.min(bundleA.size, bundleB.size));

  return overlapScore * sizeScore;
}

export function findMergeCandidates(
  bundleGraph: IdealBundleGraph,
  bundles: Array<NodeId>,
  threshold: number,
) {
  console.time('findMergeCandidates');
  let graph = new ContentGraph<NodeId>();
  let seen = new Set<string>();
  let candidates = new Set<NodeId>();

  // Build graph of clustered merge candidates
  for (let bundleId of bundles) {
    let bundle = bundleGraph.getNode(bundleId);
    invariant(bundle && bundle !== 'root');
    if (bundle.type !== 'js') {
      continue;
    }

    for (let otherBundleId of bundles) {
      if (bundleId === otherBundleId) {
        continue;
      }

      let key = [bundleId, otherBundleId].sort().join(':');

      if (seen.has(key)) {
        continue;
      }
      seen.add(key);

      let otherBundle = bundleGraph.getNode(otherBundleId);
      invariant(otherBundle && otherBundle !== 'root');

      let score = scoreBundleMerge(bundle, otherBundle);

      if (score >= threshold) {
        let bundleNode = graph.addNodeByContentKeyIfNeeded(
          bundleId.toString(),
          bundleId,
        );
        let otherBundleNode = graph.addNodeByContentKeyIfNeeded(
          otherBundleId.toString(),
          otherBundleId,
        );

        // Add edge in both directions
        graph.addEdge(bundleNode, otherBundleNode);
        graph.addEdge(otherBundleNode, bundleNode);

        candidates.add(bundleNode);
        candidates.add(otherBundleNode);
      }
    }
  }

  const clusters: Array<Array<NodeId>> = [];

  for (let candidate of candidates) {
    let cluster: Array<NodeId> = [];

    graph.traverse((nodeId) => {
      cluster.push(graph.getNode(nodeId));
      // Remove node from candidates as it has already been processed
      candidates.delete(nodeId);
    }, candidate);

    clusters.push(cluster);
  }

  clusters.sort((a, b) => b.length - a.length);

  let firstCluster = clusters[0];
  let allSourceBundles = new Set();
  let mergedBundleSize = 0;

  for (let bundleId of firstCluster) {
    let bundle = bundleGraph.getNode(bundleId);

    invariant(bundle && bundle !== 'root');

    mergedBundleSize += bundle.size;

    for (let sourceBundle of bundle.sourceBundles) {
      allSourceBundles.add(sourceBundle);
    }
  }

  console.log('Merged bundle size', mergedBundleSize);
  console.log('Number of source bundles', allSourceBundles.size);

  for (let bundleId of firstCluster) {
    let bundle = bundleGraph.getNode(bundleId);

    invariant(bundle && bundle !== 'root');

    console.table({
      bundleId,
      size: bundle.size,
      sourceBundles: bundle.sourceBundles.size,
      type: bundle.type,
      overlapScore: bundle.sourceBundles.size / allSourceBundles.size,
    });
  }

  console.timeEnd('findMergeCandidates');
  console.log('Clusters', clusters);
}
