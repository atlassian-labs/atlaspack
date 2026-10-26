"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.Stats = void 0;
function _path() {
  const data = require("path");
  _path = function () {
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
class Stats {
  merges = new (_utils().DefaultMap)(() => []);
  constructor(projectRoot) {
    this.projectRoot = projectRoot;
  }
  trackMerge(bundleToKeep, bundleToRemove, reason) {
    if (!_utils().debugTools['bundle-stats']) {
      return;
    }
    this.merges.get(bundleToKeep).push(...this.merges.get(bundleToRemove), {
      id: bundleToRemove,
      reason
    });
    this.merges.delete(bundleToRemove);
  }
  getBundleLabel(bundle) {
    if (bundle.manualSharedBundle) {
      return bundle.manualSharedBundle;
    }
    if (bundle.mainEntryAsset) {
      let relativePath = (0, _path().relative)(this.projectRoot, bundle.mainEntryAsset.filePath);
      if (relativePath.length > 100) {
        relativePath = relativePath.slice(0, 50) + '...' + relativePath.slice(-50);
      }
      return relativePath;
    }
    return `shared`;
  }
  report(getBundle) {
    if (!_utils().debugTools['bundle-stats']) {
      return;
    }
    let mergeResults = [];
    let totals = {
      label: 'Totals',
      merges: 0
    };
    for (let [bundleId, mergedBundles] of this.merges) {
      let bundle = getBundle(bundleId);
      if (!bundle) {
        continue;
      }
      let result = {
        label: this.getBundleLabel(bundle),
        size: bundle.size,
        merges: mergedBundles.length
      };
      for (let merged of mergedBundles) {
        result[merged.reason] = (result[merged.reason] || 0) + 1;
        totals[merged.reason] = (totals[merged.reason] || 0) + 1;
      }
      totals.merges += mergedBundles.length;
      mergeResults.push(result);
    }
    mergeResults.sort((a, b) => {
      // Sort by bundle size descending
      return b.size - a.size;
    });
    mergeResults.push(totals);

    // eslint-disable-next-line no-console
    console.table(mergeResults);
  }
}
exports.Stats = Stats;