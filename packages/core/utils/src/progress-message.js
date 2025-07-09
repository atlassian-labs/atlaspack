// @flow strict-local
import type {BuildProgressEvent} from '@atlaspack/types';

import path from 'path';

export function getProgressMessage(event: BuildProgressEvent): ?string {
  switch (event.phase) {
    case 'transforming':
      return `Building ${path.basename(event.filePath)}...`;

    case 'bundling':
      return 'Bundling...';

    case 'packaging':
      return `Packaging ${event.bundle.displayName}...`;

    case 'optimizing':
      return `Optimizing ${event.bundle.displayName}...`;

    case 'packagingAndOptimizing': {
      let percent = Math.floor(
        (event.completeBundles / event.totalBundles) * 100,
      );

      return `Packaging bundles ${event.completeBundles} / ${event.totalBundles} (${percent}%)`;
    }
  }

  return null;
}
