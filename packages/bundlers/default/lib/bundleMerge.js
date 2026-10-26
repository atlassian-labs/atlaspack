"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.findMergeCandidates = findMergeCandidates;
function _assert() {
  const data = _interopRequireDefault(require("assert"));
  _assert = function () {
    return data;
  };
  return data;
}
function _nullthrows() {
  const data = _interopRequireDefault(require("nullthrows"));
  _nullthrows = function () {
    return data;
  };
  return data;
}
function _graph() {
  const data = require("@atlaspack/graph");
  _graph = function () {
    return data;
  };
  return data;
}
function _utils() {
  const data = require("@atlaspack/utils");
  _utils = function () {
    return data;
  };
  return data;
}
var _memoize = require("./memoize");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { default: e }; }
function getBundlesForBundleGroup(bundleGraph, bundleGroupId) {
  let count = 0;
  bundleGraph.traverse(nodeId => {
    const node = bundleGraph.getNode(nodeId);
    if (node && (node === 'root' || node.bundleBehavior !== 'inline' && node.bundleBehavior !== 'inlineIsolated')) {
      count++;
    }
  }, bundleGroupId);
  return count;
}
let getBundleOverlap = (sourceBundlesA, sourceBundlesB) => {
  let allSourceBundles = (0, _utils().setUnion)(sourceBundlesA, sourceBundlesB);
  let sharedSourceBundles = (0, _utils().setIntersectStatic)(sourceBundlesA, sourceBundlesB);
  return sharedSourceBundles.size / allSourceBundles.size;
};

// Returns a decimal showing the proportion source bundles are common to
// both bundles versus the total number of source bundles.
function checkBundleThreshold(bundleA, bundleB, threshold) {
  return getBundleOverlap(bundleA.bundle.sourceBundles, bundleB.bundle.sourceBundles) >= threshold;
}
let checkSharedSourceBundles = (0, _memoize.memoize)((bundle, importantAncestorBundles) => {
  return importantAncestorBundles.every(ancestorId => bundle.sourceBundles.has(ancestorId));
});
let hasSuitableBundleGroup = (0, _memoize.memoize)((bundleGraph, bundle, minBundlesInGroup) => {
  for (let sourceBundle of bundle.sourceBundles) {
    let bundlesInGroup = getBundlesForBundleGroup(bundleGraph, sourceBundle);
    if (bundlesInGroup >= minBundlesInGroup) {
      return true;
    }
  }
  return false;
});
function validMerge(bundleGraph, config, bundleA, bundleB) {
  if (config.maxBundleSize != null) {
    if (bundleA.bundle.size > config.maxBundleSize || bundleB.bundle.size > config.maxBundleSize) {
      return false;
    }
  }
  if (config.overlapThreshold != null) {
    if (!checkBundleThreshold(bundleA, bundleB, config.overlapThreshold)) {
      return false;
    }
  }
  if (config.sourceBundles != null) {
    if (!checkSharedSourceBundles(bundleA.bundle, config.sourceBundles) || !checkSharedSourceBundles(bundleB.bundle, config.sourceBundles)) {
      return false;
    }
  }
  if (config.minBundlesInGroup != null) {
    if (!hasSuitableBundleGroup(bundleGraph, bundleA.bundle, config.minBundlesInGroup) || !hasSuitableBundleGroup(bundleGraph, bundleB.bundle, config.minBundlesInGroup)) {
      return false;
    }
  }
  return true;
}
function getMergeClusters(graph, candidates) {
  let clusters = [];
  for (let [candidate, edgeType] of candidates.entries()) {
    let cluster = [];
    graph.traverse(nodeId => {
      cluster.push((0, _nullthrows().default)(graph.getNode(nodeId)));
      // Remove node from candidates as it has already been processed
      candidates.delete(nodeId);
    }, candidate, edgeType);
    clusters.push(cluster);
  }
  return clusters;
}
function getPossibleMergeCandidates(bundleGraph, bundles) {
  let mergeCandidates = bundles.map(bundleId => {
    let bundle = bundleGraph.getNode(bundleId);
    (0, _assert().default)(bundle && bundle !== 'root', 'Bundle should exist');
    return {
      id: bundleId,
      bundle,
      contentKey: bundleId.toString()
    };
  });
  const uniquePairs = [];
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
function findMergeCandidates(bundleGraph, bundles, config) {
  let graph = new (_graph().ContentGraph)();
  let candidates = new Map();
  let allPossibleMergeCandidates = getPossibleMergeCandidates(bundleGraph, bundles);

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
      let bundleNode = graph.addNodeByContentKeyIfNeeded(candidateA.contentKey, candidateA.id);
      let otherBundleNode = graph.addNodeByContentKeyIfNeeded(candidateB.contentKey, candidateB.id);

      // Add edge in both directions
      graph.addEdge(bundleNode, otherBundleNode, edgeType);
      graph.addEdge(otherBundleNode, bundleNode, edgeType);
      candidates.set(bundleNode, edgeType);
      candidates.set(otherBundleNode, edgeType);
    }

    // Remove bundles that have been allocated to a higher priority merge
    allPossibleMergeCandidates = allPossibleMergeCandidates.filter(group => !graph.hasContentKey(group[0].contentKey) && !graph.hasContentKey(group[1].contentKey));
  }
  (0, _memoize.clearCaches)();
  return getMergeClusters(graph, candidates);
}