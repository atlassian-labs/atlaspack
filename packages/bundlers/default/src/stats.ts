import {relative} from 'path';
import {NodeId} from '@atlaspack/graph';
import {DefaultMap, debugTools} from '@atlaspack/utils';

import {Bundle} from './idealGraph';

interface MergedBundle {
  id: NodeId;
  reason: string;
}

export class Stats {
  projectRoot: string;
  merges: DefaultMap<NodeId, MergedBundle[]> = new DefaultMap(() => []);

  constructor(projectRoot: string) {
    this.projectRoot = projectRoot;
  }

  trackMerge(bundleToKeep: NodeId, bundleToRemove: NodeId, reason: string) {
    if (!debugTools['bundle-stats']) {
      return;
    }

    this.merges
      .get(bundleToKeep)
      .push(...this.merges.get(bundleToRemove), {id: bundleToRemove, reason});
    this.merges.delete(bundleToRemove);
  }

  getBundleLabel(bundle: Bundle): string {
    if (bundle.manualSharedBundle) {
      return bundle.manualSharedBundle;
    }

    if (bundle.mainEntryAsset) {
      let relativePath = relative(
        this.projectRoot,
        bundle.mainEntryAsset.filePath,
      );

      if (relativePath.length > 100) {
        relativePath =
          relativePath.slice(0, 50) + '...' + relativePath.slice(-50);
      }

      return relativePath;
    }

    return `shared`;
  }

  report(getBundle: (bundleId: NodeId) => Bundle | null | undefined): void {
    if (!debugTools['bundle-stats']) {
      return;
    }

    type MergeResult = Record<string, string | number>;
    let mergeResults: Array<MergeResult> = [];

    let totals: Record<string, string | number> = {
      label: 'Totals',
      merges: 0,
    };

    for (let [bundleId, mergedBundles] of this.merges) {
      let bundle = getBundle(bundleId);
      if (!bundle) {
        continue;
      }

      let result: MergeResult = {
        label: this.getBundleLabel(bundle),
        size: bundle.size,
        merges: mergedBundles.length,
      };

      for (let merged of mergedBundles) {
        result[merged.reason] = ((result[merged.reason] as number) || 0) + 1;
        totals[merged.reason] = ((totals[merged.reason] as number) || 0) + 1;
      }

      (totals.merges as number) += mergedBundles.length;
      mergeResults.push(result);
    }

    mergeResults.sort((a, b) => {
      // Sort by bundle size descending
      return (b.size as number) - (a.size as number);
    });

    mergeResults.push(totals);

    // eslint-disable-next-line no-console
    console.table(mergeResults);
  }
}
