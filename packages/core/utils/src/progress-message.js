// @flow strict-local
import type {BuildProgressEvent} from '@atlaspack/types';

import path from 'path';

export function getProgressMessage(event: BuildProgressEvent): ?string {
  switch (event.phase) {
    case 'transforming':
      return `Building ${event.filePath} (${event.phase})...`;

    case 'bundling':
      return 'Bundling...';

    case 'packaging':
      return `Packaging ${event.bundle.displayName}...`;

    case 'optimizing':
      return `Optimizing ${event.bundle.displayName}...`;
  }

  return null;
}
