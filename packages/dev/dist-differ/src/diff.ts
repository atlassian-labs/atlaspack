import type {DiffEntry} from './types';
import {
  linesDifferOnlyByAssetIds,
  linesDifferOnlyByUnminifiedRefs,
} from './normalize';

/**
 * Computes a simple line-by-line diff between two arrays of lines
 */
export function computeDiff(lines1: string[], lines2: string[]): DiffEntry[] {
  const diff: DiffEntry[] = [];
  const maxLen = Math.max(lines1.length, lines2.length);

  let i = 0;
  while (i < maxLen) {
    if (i >= lines1.length) {
      // Lines only in file2 (additions)
      diff.push({type: 'add', line: lines2[i], lineNum2: i + 1});
      i++;
    } else if (i >= lines2.length) {
      // Lines only in file1 (deletions)
      diff.push({type: 'remove', line: lines1[i], lineNum1: i + 1});
      i++;
    } else if (lines1[i] === lines2[i]) {
      // Lines match
      diff.push({
        type: 'equal',
        line: lines1[i],
        lineNum1: i + 1,
        lineNum2: i + 1,
      });
      i++;
    } else {
      // Lines differ
      diff.push({
        type: 'remove',
        line: lines1[i],
        lineNum1: i + 1,
      });
      diff.push({
        type: 'add',
        line: lines2[i],
        lineNum2: i + 1,
      });
      i++;
    }
  }

  return diff;
}
