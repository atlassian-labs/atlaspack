// @flow strict-local

import invariant from 'assert';
import path from 'path';
import {basename} from 'path';

import nullthrows from 'nullthrows';

import type {NodeId} from '@atlaspack/graph';
import type {Asset, Dependency, MutableBundleGraph} from '@atlaspack/types';

import type {Bundle, IdealBundleGraph} from './idealGraph';
import {inspect} from 'util';
/* $FlowFixMe[untyped-import] */
import {PriorityQueue} from './PriorityQueue';
import type {DefaultMap} from '@atlaspack/utils';

/** 100Kb */
const MAX_SHARED_BUNDLE_SIZE = 100e3;

/**
 * @returns The sum of the size of all assets that will be loaded
 * in sourceBundles that weren't previously loaded before
 * merging `bundleA` into `bundleB`
 */
function getNewAssetsLoadedByMerge(
  bundleGraph: IdealBundleGraph,
  assetReference: DefaultMap<Asset, Array<[Dependency, Bundle]>>,
  bundleA: Bundle,
  bundleB: Bundle,
) {
  /**
   * @returns The sum of the size of all assets that will be loaded
   * in `bundleB.sourceBundles` that weren't previously loaded before
   * merging `bundleA` into `bundleB`
   */
  function getNewAssetsLoadedInBByMerge(bundleA: Bundle, bundleB: Bundle) {
    const sourceBundlesUniqueToB = new Set();
    for (const id of bundleB.sourceBundles) {
      if (!bundleA.sourceBundles.has(id)) {
        const bundle = nullthrows(bundleGraph.getNode(id));
        if (bundle !== 'root') {
          sourceBundlesUniqueToB.add(bundle);
        }
      }
    }

    const assetsUniqueToA = new Set();
    for (const asset of bundleA.assets) {
      if (!bundleB.assets.has(asset)) {
        assetsUniqueToA.add(asset);
      }
    }

    let duplicatedAssetsFromA = 0;
    for (const sourceBundle of sourceBundlesUniqueToB) {
      for (const asset of assetsUniqueToA) {
        if (sourceBundle.assets.has(asset)) {
          continue;
        }
        const references = nullthrows(assetReference.get(asset));
        const sourceBundleReferencesAsset = references.some(
          (_dependency, bundle) => bundle === sourceBundle,
        );
        if (!sourceBundleReferencesAsset) {
          duplicatedAssetsFromA += asset.stats.size;
        }
      }
    }

    return duplicatedAssetsFromA;
  }

  /**
   * The sum of the sizes of new assets being loaded in `sourceBundles` unique to `bundleB`
   * that weren't previousely being loaded before `bundleA` merges into `bundleB`
   */
  const newAssetsLoadedInBFromA = getNewAssetsLoadedInBByMerge(
    bundleA,
    bundleB,
  );
  /**
   * The sum of the sizes of new assets being loaded in `sourceBundles` unique to `bundleA`
   * that weren't previousely being loaded before `bundleA` merges into `bundleB`
   */
  const newAssetsLoadedInAFromB = getNewAssetsLoadedInBByMerge(
    bundleB,
    bundleA,
  );

  return newAssetsLoadedInBFromA + newAssetsLoadedInAFromB;
}

function getBundleSizeAfterMerge(bundleA: Bundle, bundleB: Bundle) {
  const combinedAssets = new Set([...bundleA.assets, ...bundleB.assets]);

  let bundleSizeAfterMerge = 0;
  for (const asset of combinedAssets) {
    bundleSizeAfterMerge += asset.stats.size;
  }

  return bundleSizeAfterMerge;
}

/**
 * Create a `compareFn` used for determining if a merge
 * (`[bundleA, bundleB]` where `bundleA` is being merged into `bundleB`)
 *
 *  1. Is merging a smaller asset (`bundleA`)
 *  2. Leads to less new code loaded after merge
 */
function createSortSmallestMergeByLeastCodeLoaded(
  bundleGraph: IdealBundleGraph,
  assetReference: DefaultMap<Asset, Array<[Dependency, Bundle]>>,
) {
  [].sort();
  function getBundle(nodeId: NodeId) {
    const bundle = bundleGraph.getNode(nodeId);
    invariant(bundle && bundle !== 'root');
    return bundle;
  }

  function isMergeOutdated(merge: [NodeId, NodeId]) {
    return merge.some((nodeId) => !bundleGraph.hasNode(nodeId));
  }

  return (mergeA: [NodeId, NodeId], mergeB: [NodeId, NodeId]) => {
    /**
     * These invalid merges will be removed by the `isCandidateInvalid`
     * check in `shift`
     */
    if (isMergeOutdated(mergeA)) {
      return 1;
    } else if (isMergeOutdated(mergeB)) {
      return -1;
    }

    const mergeBundlesA = mergeA.map(getBundle);
    const mergeBundlesB = mergeB.map(getBundle);

    const bundleSizeAfterMergeA = getBundleSizeAfterMerge(...mergeBundlesA);
    const bundleSizeAfterMergeB = getBundleSizeAfterMerge(...mergeBundlesB);

    // We want to prevent overmerging assets!!
    if (bundleSizeAfterMergeA > MAX_SHARED_BUNDLE_SIZE) {
      return 1;
    } else if (bundleSizeAfterMergeB > MAX_SHARED_BUNDLE_SIZE) {
      return -1;
    }

    const getNewAssetsLoadedAfterMergeA = getNewAssetsLoadedByMerge(
      bundleGraph,
      assetReference,
      ...mergeBundlesA,
    );
    const getNewAssetsLoadedAfterMergeB = getNewAssetsLoadedByMerge(
      bundleGraph,
      assetReference,
      ...mergeBundlesB,
    );

    return getNewAssetsLoadedAfterMergeA - getNewAssetsLoadedAfterMergeB;
  };
}

/**
 * @param {{ id: NodeId, bundle: Bundle }[]} sharedBundles all sharedBundles within `bundleGroupId`
 * @returns A PriorityQueue of shared bundle merge combinations within `bundleGroupId` which lead
 * to less code being loaded when compared to merging the bundle back into the the `bundleGroup`.
 *
 * The queue is sorted by:
 *  1. Smallest bundles merged first
 *  2. Merge combinations which lead to the least amount of new code loaded after being merged
 */
export function getSmallestSharedBundleMergesByLeastCodeLoaded(
  sharedBundles: {|id: NodeId, bundle: Bundle|}[],
  bundleGraph: IdealBundleGraph,
  bundleGroupId: NodeId,
  assetReference: DefaultMap<Asset, Array<[Dependency, Bundle]>>,
): {|
  shift():
    | typeof undefined
    | {|bundleToMergeId: NodeId, bundleToKeepId: NodeId|},
|} {
  const seen = new Set<string>();
  const queue = new PriorityQueue(
    createSortSmallestMergeByLeastCodeLoaded(bundleGraph, assetReference),
  );
  // Only consider JS shared bundles and non-reused bundles.
  // These could potentially be considered for merging in future but they're
  // more complicated to merge
  const nonReusedSharedBundles = sharedBundles.filter(
    ({bundle}) => bundle.type === 'js' && !bundle.mainEntryAsset,
  );

  const bundleGroup = nullthrows(bundleGraph.getNode(bundleGroupId));
  invariant(bundleGroup !== 'root');

  function isCandidateInvalid(
    [bundleId, otherBundleId]: [NodeId, NodeId],
    newCodeLoadedAfterBundleGroupMerge?: number,
  ) {
    if (!bundleGraph.hasNode(bundleId) || !bundleGraph.hasNode(otherBundleId)) {
      return true;
    }

    const bundle = nullthrows(bundleGraph.getNode(bundleId));
    invariant(bundle !== 'root');
    const otherBundle = nullthrows(bundleGraph.getNode(otherBundleId));
    invariant(otherBundle !== 'root');

    const bundleSizeAfterMerge = getBundleSizeAfterMerge(bundle, otherBundle);
    if (bundleSizeAfterMerge > MAX_SHARED_BUNDLE_SIZE) {
      return true;
    }

    const newCodeLoadedAfterOtherBundleMerge = getNewAssetsLoadedByMerge(
      bundleGraph,
      assetReference,
      bundle,
      otherBundle,
    );
    /* $FlowIssue[reassign-const] Flow thinks that parameters are consts */
    newCodeLoadedAfterBundleGroupMerge ??= getNewAssetsLoadedByMerge(
      bundleGraph,
      assetReference,
      bundle,
      bundleGroup,
    );
    return (
      /* $FlowIssue[invalid-compare] duplicatedAssetsFromOtherBundleMerge will always be a number here */
      newCodeLoadedAfterOtherBundleMerge >= newCodeLoadedAfterBundleGroupMerge
    );
  }

  for (const {id: bundleId, bundle} of nonReusedSharedBundles) {
    const newCodeLoadedAfterBundleGroupMerge = getNewAssetsLoadedByMerge(
      bundleGraph,
      assetReference,
      bundle,
      bundleGroup,
    );
    for (const {
      id: otherBundleId,
      bundle: otherBundle,
    } of nonReusedSharedBundles) {
      if (bundleId === otherBundleId) {
        continue;
      }
      let key = [bundleId, otherBundleId].sort().join(':');
      if (seen.has(key)) {
        continue;
      }

      const candidate = [bundleId, otherBundleId];
      if (!isCandidateInvalid(candidate, newCodeLoadedAfterBundleGroupMerge)) {
        queue.push(candidate);
      }
    }
  }

  return {
    shift() {
      let candidate = queue.pop();
      while (candidate && isCandidateInvalid(candidate)) {
        candidate = queue.pop();
      }

      if (!candidate) {
        return undefined;
      }

      const [bundleToMergeId, bundleToKeepId] = candidate;
      return {bundleToMergeId, bundleToKeepId};
    },
  };
}
