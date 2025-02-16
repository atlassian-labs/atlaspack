// @flow strict-local

import path from 'path';
import {type PackagingInputGraph, getAssetNodeByKey} from '../src';

export function testMakePackageKey(
  entryDir: string,
  dominators: PackagingInputGraph,
  parentChunks: Set<string>,
): string {
  if (parentChunks.size === 0) {
    return 'root';
  }

  const chunks = Array.from(parentChunks);
  const chunkPaths = chunks.map((chunk) =>
    path.relative(entryDir, getAssetNodeByKey(dominators, chunk).filePath),
  );
  chunkPaths.sort((a, b) => a.localeCompare(b));
  return chunkPaths.join(',');
}
