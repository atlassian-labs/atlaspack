import type {DiffEntry} from './types';
import {
  linesDifferOnlyByAssetIds,
  linesDifferOnlyByUnminifiedRefs,
  linesDifferOnlyBySourceMapUrl,
  linesDifferOnlyBySwappedVariables,
  normalizeUnminifiedRefs,
} from './normalize';

/**
 * Filters out individual pairs that differ only by asset IDs, unminified refs, source map URLs, or swapped variables
 * Returns the filtered entries and updated counts
 */
export function filterHunkEntries(
  hunkEntries: DiffEntry[],
  ignoreAssetIds: boolean,
  ignoreUnminifiedRefs: boolean,
  ignoreSourceMapUrl: boolean = false,
  ignoreSwappedVariables: boolean = false,
): {filtered: DiffEntry[]; removeCount: number; addCount: number} {
  if (
    !ignoreAssetIds &&
    !ignoreUnminifiedRefs &&
    !ignoreSourceMapUrl &&
    !ignoreSwappedVariables
  ) {
    return {
      filtered: hunkEntries,
      removeCount: hunkEntries.filter((e) => e.type === 'remove').length,
      addCount: hunkEntries.filter((e) => e.type === 'add').length,
    };
  }

  const filtered: DiffEntry[] = [];
  const paired = new Set<number>();

  // Pair up removes and adds, keeping only pairs with real differences
  for (let i = 0; i < hunkEntries.length; i++) {
    const entry = hunkEntries[i];
    if (paired.has(i)) continue;

    if (entry.type === 'remove') {
      // Find the first unpaired add entry after this remove
      let addIdx = -1;
      for (let j = i + 1; j < hunkEntries.length; j++) {
        if (hunkEntries[j].type === 'add' && !paired.has(j)) {
          addIdx = j;
          break;
        }
      }
      if (addIdx >= 0) {
        const addEntry = hunkEntries[addIdx];
        // Check if this pair should be filtered
        let shouldFilter = false;
        if (
          ignoreAssetIds &&
          linesDifferOnlyByAssetIds(entry.line, addEntry.line)
        ) {
          shouldFilter = true;
        }
        if (
          ignoreUnminifiedRefs &&
          !shouldFilter &&
          linesDifferOnlyByUnminifiedRefs(entry.line, addEntry.line)
        ) {
          shouldFilter = true;
        }
        if (
          ignoreSourceMapUrl &&
          !shouldFilter &&
          linesDifferOnlyBySourceMapUrl(entry.line, addEntry.line)
        ) {
          shouldFilter = true;
        }
        if (
          ignoreSwappedVariables &&
          !shouldFilter &&
          linesDifferOnlyBySwappedVariables(entry.line, addEntry.line)
        ) {
          shouldFilter = true;
        }

        if (!shouldFilter) {
          // Keep this pair - it has real differences
          filtered.push(entry);
          filtered.push(addEntry);
        }
        paired.add(i);
        paired.add(addIdx);
      } else {
        // Orphaned remove - keep it
        filtered.push(entry);
        paired.add(i);
      }
    } else if (entry.type === 'add') {
      // Find the first unpaired remove entry before this add
      let removeIdx = -1;
      for (let j = i - 1; j >= 0; j--) {
        if (hunkEntries[j].type === 'remove' && !paired.has(j)) {
          removeIdx = j;
          break;
        }
      }
      if (removeIdx >= 0) {
        // Already handled in the remove case above
        continue;
      } else {
        // Orphaned add - keep it
        filtered.push(entry);
        paired.add(i);
      }
    }
  }

  const filteredRemoves = filtered.filter((e) => e.type === 'remove');
  const filteredAdds = filtered.filter((e) => e.type === 'add');

  return {
    filtered,
    removeCount: filteredRemoves.length,
    addCount: filteredAdds.length,
  };
}

/**
 * Generic function to check if a hunk consists only of a specific type of difference
 */
function isHunkOnlyByType(
  hunkEntries: DiffEntry[],
  differOnlyBy: (line1: string, line2: string) => boolean,
): boolean {
  // A hunk consists of remove/add pairs
  // Check if all pairs differ only by the specified type
  for (let i = 0; i < hunkEntries.length; i++) {
    const entry = hunkEntries[i];
    if (entry.type === 'remove') {
      // Find the corresponding add entry
      const addEntry = hunkEntries.find(
        (e, idx) => idx > i && e.type === 'add',
      );
      if (addEntry) {
        // Check if this pair differs only by the specified type
        if (!differOnlyBy(entry.line, addEntry.line)) {
          return false;
        }
      } else {
        // Orphaned remove (no corresponding add) - not just this type
        return false;
      }
    } else if (entry.type === 'add') {
      // Find the corresponding remove entry
      const removeEntry = hunkEntries.find(
        (e, idx) => idx < i && e.type === 'remove',
      );
      if (!removeEntry) {
        // Orphaned add (no corresponding remove) - not just this type
        return false;
      }
      // Already checked in the remove case above
    }
  }
  return true;
}

/**
 * Checks if a hunk consists only of asset ID differences
 */
export function isHunkOnlyAssetIds(hunkEntries: DiffEntry[]): boolean {
  return isHunkOnlyByType(hunkEntries, linesDifferOnlyByAssetIds);
}

/**
 * Checks if a hunk consists only of swapped variable differences
 */
export function isHunkOnlySwappedVariables(hunkEntries: DiffEntry[]): boolean {
  return isHunkOnlyByType(hunkEntries, linesDifferOnlyBySwappedVariables);
}

/**
 * Checks if a hunk consists only of source map URL differences
 */
export function isHunkOnlySourceMapUrl(hunkEntries: DiffEntry[]): boolean {
  return isHunkOnlyByType(hunkEntries, linesDifferOnlyBySourceMapUrl);
}

/**
 * Checks if a hunk consists only of unminified ref differences
 */
export function isHunkOnlyUnminifiedRefs(hunkEntries: DiffEntry[]): boolean {
  // A hunk consists of remove/add pairs
  // Check if all pairs differ only by unminified refs
  // Strategy: Count normalized lines - if the counts match, all differences are just unminified refs
  const removes = hunkEntries.filter((e) => e.type === 'remove');
  const adds = hunkEntries.filter((e) => e.type === 'add');

  // If counts don't match, it's not just unminified refs
  if (removes.length !== adds.length) {
    return false;
  }

  // Quick check: if no line contains the pattern, skip expensive normalization
  let hasPattern = false;
  for (const entry of [...removes, ...adds]) {
    if (entry.line.includes('$exports') || entry.line.includes('$var$')) {
      hasPattern = true;
      break;
    }
  }
  if (!hasPattern) {
    return false;
  }

  // Count normalized lines using a Map (faster than sorting)
  const removeCounts = new Map<string, number>();
  for (const entry of removes) {
    const normalized = normalizeUnminifiedRefs(entry.line);
    removeCounts.set(normalized, (removeCounts.get(normalized) || 0) + 1);
  }

  const addCounts = new Map<string, number>();
  for (const entry of adds) {
    const normalized = normalizeUnminifiedRefs(entry.line);
    addCounts.set(normalized, (addCounts.get(normalized) || 0) + 1);
  }

  // Check if counts match
  if (removeCounts.size !== addCounts.size) {
    return false;
  }

  for (const [normalized, count] of removeCounts) {
    if (addCounts.get(normalized) !== count) {
      return false;
    }
  }

  return true;
}

/**
 * Counts the number of hunks in a diff, optionally filtering by asset IDs, unminified refs, source map URLs, or swapped variables
 */
export function countHunks(
  diff: DiffEntry[],
  ignoreAssetIds: boolean = false,
  ignoreUnminifiedRefs: boolean = false,
  ignoreSourceMapUrl: boolean = false,
  ignoreSwappedVariables: boolean = false,
): number {
  let hunkCount = 0;
  let inChangeBlock = false;
  let currentHunk: DiffEntry[] = [];

  for (let i = 0; i < diff.length; i++) {
    const entry = diff[i];

    if (entry.type === 'equal') {
      if (inChangeBlock) {
        // We've reached the end of a hunk, check if we should count it
        let shouldSkipHunk = false;
        if (ignoreAssetIds && currentHunk.length > 0) {
          shouldSkipHunk = isHunkOnlyAssetIds(currentHunk);
        }
        if (ignoreUnminifiedRefs && currentHunk.length > 0 && !shouldSkipHunk) {
          shouldSkipHunk = isHunkOnlyUnminifiedRefs(currentHunk);
        }
        if (ignoreSourceMapUrl && currentHunk.length > 0 && !shouldSkipHunk) {
          shouldSkipHunk = isHunkOnlySourceMapUrl(currentHunk);
        }
        if (
          ignoreSwappedVariables &&
          currentHunk.length > 0 &&
          !shouldSkipHunk
        ) {
          shouldSkipHunk = isHunkOnlySwappedVariables(currentHunk);
        }

        if (!shouldSkipHunk) {
          // Filter individual pairs and only count if there are real differences
          const {filtered} = filterHunkEntries(
            currentHunk,
            ignoreAssetIds,
            ignoreUnminifiedRefs,
            ignoreSourceMapUrl,
            ignoreSwappedVariables,
          );
          if (filtered.length > 0) {
            hunkCount++;
          }
        }

        currentHunk = [];
        inChangeBlock = false;
      }
    } else {
      if (!inChangeBlock) {
        inChangeBlock = true;
      }
      currentHunk.push(entry);
    }
  }

  // Handle any remaining hunk at the end
  if (currentHunk.length > 0) {
    let shouldSkipHunk = false;
    if (ignoreAssetIds) {
      shouldSkipHunk = isHunkOnlyAssetIds(currentHunk);
    }
    if (ignoreUnminifiedRefs && !shouldSkipHunk) {
      shouldSkipHunk = isHunkOnlyUnminifiedRefs(currentHunk);
    }
    if (ignoreSourceMapUrl && !shouldSkipHunk) {
      shouldSkipHunk = isHunkOnlySourceMapUrl(currentHunk);
    }
    if (ignoreSwappedVariables && !shouldSkipHunk) {
      shouldSkipHunk = isHunkOnlySwappedVariables(currentHunk);
    }

    if (!shouldSkipHunk) {
      // Filter individual pairs and only count if there are real differences
      const {filtered} = filterHunkEntries(
        currentHunk,
        ignoreAssetIds,
        ignoreUnminifiedRefs,
        ignoreSourceMapUrl,
        ignoreSwappedVariables,
      );
      if (filtered.length > 0) {
        hunkCount++;
      }
    }
  }

  return hunkCount;
}
