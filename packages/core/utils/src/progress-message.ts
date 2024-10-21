import type {BuildProgressEvent} from '@atlaspack/types';

import path from 'path';

export function getProgressMessage(event: BuildProgressEvent): string | null | undefined {
  switch (event.phase) {
    case 'transforming':
      return `Building ${path.basename(event.filePath)}...`;

    case 'bundling':
      return 'Bundling...';

    case 'packaging':
      return `Packaging ${event.bundle.displayName}...`;

    case 'optimizing':
      return `Optimizing ${event.bundle.displayName}...`;
  }

  return null;
}
