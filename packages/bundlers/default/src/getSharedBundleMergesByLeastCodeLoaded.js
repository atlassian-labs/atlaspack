// @flow strict-local

import invariant from 'assert';

import nullthrows from 'nullthrows';

import type {NodeId} from '@atlaspack/graph';
import type {Asset, Dependency} from '@atlaspack/types';
import type {DefaultMap} from '@atlaspack/utils';

import type {Bundle, IdealBundleGraph} from './idealGraph';
import {PriorityQueue} from './PriorityQueue';

/** 100Kb */
const MAX_SHARED_BUNDLE_SIZE = 100e3;

/**
 * @returns The of the size of all assets that will be created
 * if `bundle` is duplicated into all of the `sourceBundles` that
 * it exists in.
 */
function getNewAssetsLoadedByBundleGroupMerge(bundle: Bundle): number {
  return bundle.size * (bundle.sourceBundles.size - 1);
}

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
   * merging `bundleA` and `bundleB`
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

    let newAssetSizeLoadedInBFromA = 0;
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
          newAssetSizeLoadedInBFromA += asset.stats.size;
        }
      }
    }

    return newAssetSizeLoadedInBFromA;
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
 * @returns A `compareFn` which determinines if a `mergeA`
 * leads to less code loaded when compared to `mergeB`
 */
function getCompareBundleMergesByCodeLoaded(
  bundleGraph: IdealBundleGraph,
  assetReference: DefaultMap<Asset, Array<[Dependency, Bundle]>>,
) {
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
     * These invalid merges will be removed by the `isMergeInvalid`
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

    // We want to prevent over-merging assets!!
    if (bundleSizeAfterMergeA > MAX_SHARED_BUNDLE_SIZE) {
      return 1;
    } else if (bundleSizeAfterMergeB > MAX_SHARED_BUNDLE_SIZE) {
      return -1;
    }

    const newAssetsLoadedAfterMergeA = getNewAssetsLoadedByMerge(
      bundleGraph,
      assetReference,
      ...mergeBundlesA,
    );
    const newAssetsLoadedAfterMergeB = getNewAssetsLoadedByMerge(
      bundleGraph,
      assetReference,
      ...mergeBundlesB,
    );

    return newAssetsLoadedAfterMergeA - newAssetsLoadedAfterMergeB;
  };
}

/**
 * @param {{ id: NodeId, bundle: Bundle }[]} sharedBundles all sharedBundles within `bundleGroupId`
 * @returns A PriorityQueue of shared bundle merge combinations within `bundleGroupId` which lead
 * to less code being loaded when compared to merging the bundle back into the the `bundleGroup`.
 *
 * The queue is sorted (ASC) by the amount of new code loaded by the merge.
 */
export function getSharedBundleMergesByLeastCodeLoaded(
  sharedBundles: {|id: NodeId, bundle: Bundle|}[],
  bundleGraph: IdealBundleGraph,
  assetReference: DefaultMap<Asset, Array<[Dependency, Bundle]>>,
): {|
  shift(): void | {|bundleToMergeId: NodeId, bundleToKeepId: NodeId|},
|} {
  const seen = new Set<string>();
  const queue = new PriorityQueue(
    getCompareBundleMergesByCodeLoaded(bundleGraph, assetReference),
  );
  // Only consider JS shared bundles and non-reused bundles.
  // These could potentially be considered for merging in future but they're
  // more complicated to merge
  const nonReusedSharedBundles = sharedBundles.filter(
    ({bundle}) => bundle.type === 'js' && !bundle.mainEntryAsset,
  );

  function isMergeInvalid(
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
    newCodeLoadedAfterBundleGroupMerge ??=
      getNewAssetsLoadedByBundleGroupMerge(bundle);
    return (
      /* $FlowIssue[invalid-compare] newCodeLoadedAfterOtherBundleMerge will always be a number here */
      newCodeLoadedAfterOtherBundleMerge >= newCodeLoadedAfterBundleGroupMerge
    );
  }

  for (const {id: bundleId, bundle} of nonReusedSharedBundles) {
    const newCodeLoadedAfterBundleGroupMerge =
      getNewAssetsLoadedByBundleGroupMerge(bundle);
    for (const {id: otherBundleId} of nonReusedSharedBundles) {
      if (bundleId === otherBundleId) {
        continue;
      }
      let key = [bundleId, otherBundleId].sort().join(':');
      if (seen.has(key)) {
        continue;
      }

      const merge = [bundleId, otherBundleId];
      if (!isMergeInvalid(merge, newCodeLoadedAfterBundleGroupMerge)) {
        queue.push(merge);
      }
    }
  }

  return {
    shift() {
      let merge = queue.pop();
      while (merge && isMergeInvalid(merge)) {
        merge = queue.pop();
      }

      if (!merge) {
        return undefined;
      }

      const [bundleToMergeId, bundleToKeepId] = merge;
      return {bundleToMergeId, bundleToKeepId};
    },
  };
}
