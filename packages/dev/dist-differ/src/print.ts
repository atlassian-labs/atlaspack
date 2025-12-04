/* eslint-disable no-console */
import type {DiffEntry, DiffResult} from './types';
import {getColors} from './utils/colors';
import {
  countHunks,
  filterHunkEntries,
  isHunkOnlyAssetIds,
  isHunkOnlyUnminifiedRefs,
  isHunkOnlySourceMapUrl,
  isHunkOnlySwappedVariables,
} from './hunk';

const colors = getColors();

function printDiffLine(entry: DiffEntry): void {
  const maxLineNum = 9999;
  const lineNumWidth = String(maxLineNum).length;

  switch (entry.type) {
    case 'equal': {
      // Context lines: show both line numbers if available, otherwise just one
      const lineNum1 = entry.lineNum1
        ? String(entry.lineNum1).padStart(lineNumWidth, ' ')
        : '';
      const lineNum2 = entry.lineNum2
        ? String(entry.lineNum2).padStart(lineNumWidth, ' ')
        : '';
      const lineNum = lineNum1 || lineNum2;
      console.log(`${colors.dim}  ${lineNum} ${entry.line}${colors.reset}`);
      break;
    }
    case 'remove': {
      const removeLineNum = String(entry.lineNum1 || '').padStart(
        lineNumWidth,
        ' ',
      );
      console.log(
        `${colors.red}-${removeLineNum} ${entry.line}${colors.reset}`,
      );
      break;
    }
    case 'add': {
      const addLineNum = String(entry.lineNum2 || '').padStart(
        lineNumWidth,
        ' ',
      );
      console.log(`${colors.green}+${addLineNum} ${entry.line}${colors.reset}`);
      break;
    }
  }
}

function printHunkHeader(
  start1: number,
  count1: number,
  start2: number,
  count2: number,
): void {
  console.log(
    `${colors.cyan}@@ -${start1},${count1} +${start2},${count2} @@${colors.reset}`,
  );
}

/**
 * Prints a diff with context, optionally filtering by asset IDs, unminified refs, source map URLs, or swapped variables
 */
export function printDiff(
  diff: DiffEntry[],
  file1: string,
  file2: string,
  contextLines: number = 3,
  ignoreAssetIds: boolean = false,
  ignoreUnminifiedRefs: boolean = false,
  ignoreSourceMapUrl: boolean = false,
  ignoreSwappedVariables: boolean = false,
  summaryMode: boolean = false,
): DiffResult {
  if (summaryMode) {
    const hunkCount = countHunks(
      diff,
      ignoreAssetIds,
      ignoreUnminifiedRefs,
      ignoreSourceMapUrl,
      ignoreSwappedVariables,
    );
    const hasChanges = diff.some((e) => e.type !== 'equal');
    return {hunkCount, hasChanges};
  }

  console.log(`${colors.cyan}=== Comparing files ===${colors.reset}`);
  console.log(`${colors.yellow}File 1:${colors.reset} ${file1}`);
  console.log(`${colors.yellow}File 2:${colors.reset} ${file2}`);
  console.log();

  // Group diff entries and show only changed sections with context
  let hasChanges = false;
  let hasPrintedChanges = false;
  let contextBuffer: DiffEntry[] = [];
  let inChangeBlock = false;
  let changesPrinted = 0;
  let currentHunk: DiffEntry[] = [];
  let hunkStartLine1: number | null = null;
  let hunkStartLine2: number | null = null;

  for (let i = 0; i < diff.length; i++) {
    const entry = diff[i];

    if (entry.type === 'equal') {
      if (inChangeBlock) {
        // We've reached the end of a hunk, check if we should filter it
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
          // Filter individual pairs within the hunk
          const {
            filtered: filteredHunk,
            removeCount: hunkCount1,
            addCount: hunkCount2,
          } = filterHunkEntries(
            currentHunk,
            ignoreAssetIds,
            ignoreUnminifiedRefs,
            ignoreSourceMapUrl,
            ignoreSwappedVariables,
          );

          // Only print if there are any remaining differences after filtering
          if (filteredHunk.length > 0) {
            // Print buffered context before changes (only now that we know we're showing this hunk)
            if (hasPrintedChanges) {
              console.log();
            }
            if (contextBuffer.length > 0) {
              contextBuffer.forEach(printDiffLine);
            }

            // Print hunk header
            if (hunkStartLine1 !== null && hunkStartLine2 !== null) {
              printHunkHeader(
                hunkStartLine1,
                hunkCount1,
                hunkStartLine2,
                hunkCount2,
              );
            }

            // Print the filtered hunk
            filteredHunk.forEach(printDiffLine);
            changesPrinted += filteredHunk.length;
            hasPrintedChanges = true;
          } else {
            // All pairs were filtered - skip this hunk entirely
            currentHunk = [];
            hunkStartLine1 = null;
            hunkStartLine2 = null;
            inChangeBlock = false;
            contextBuffer = [];

            // Skip past context lines
            let j = i;
            let contextCount = 0;
            while (
              j < diff.length &&
              diff[j].type === 'equal' &&
              contextCount < contextLines
            ) {
              j++;
              contextCount++;
            }

            // Check if there are more changes ahead
            let moreChanges = false;
            for (
              let k = j;
              k < Math.min(j + contextLines * 2, diff.length);
              k++
            ) {
              if (diff[k].type !== 'equal') {
                moreChanges = true;
                break;
              }
            }

            if (moreChanges) {
              i = j - 1;
            } else {
              i = j - 1;
            }
            continue;
          }

          // Print context after changes
          let contextCount = 0;
          let j = i;
          while (
            j < diff.length &&
            diff[j].type === 'equal' &&
            contextCount < contextLines
          ) {
            printDiffLine(diff[j]);
            contextCount++;
            j++;
          }

          // Check if there are more changes ahead
          let moreChanges = false;
          for (
            let k = j;
            k < Math.min(j + contextLines * 2, diff.length);
            k++
          ) {
            if (diff[k].type !== 'equal') {
              moreChanges = true;
              break;
            }
          }

          // Clear the hunk after printing
          currentHunk = [];
          hunkStartLine1 = null;
          hunkStartLine2 = null;

          if (moreChanges) {
            console.log(`${colors.dim}...${colors.reset}`);
            i = j - 1;
          } else {
            inChangeBlock = false;
            i = j - 1;
            if (i < diff.length - 1) {
              console.log();
            }
          }
        } else {
          // Hunk was filtered - skip it and don't print the buffered context either
          currentHunk = [];
          hunkStartLine1 = null;
          hunkStartLine2 = null;
          inChangeBlock = false;

          // Skip past context lines
          let j = i;
          let contextCount = 0;
          while (
            j < diff.length &&
            diff[j].type === 'equal' &&
            contextCount < contextLines
          ) {
            j++;
            contextCount++;
          }

          // Check if there are more changes ahead (that aren't filtered)
          let moreChanges = false;
          for (
            let k = j;
            k < Math.min(j + contextLines * 2, diff.length);
            k++
          ) {
            if (diff[k].type !== 'equal') {
              moreChanges = true;
              break;
            }
          }

          if (moreChanges) {
            // There might be more changes, so we'll continue
            i = j - 1;
          } else {
            // No more changes, we're done with this section
            i = j - 1;
          }
        }
        // Clear context buffer - we've either printed it or skipped it
        contextBuffer = [];
      } else {
        // Buffer context before potential changes
        contextBuffer.push(entry);
        if (contextBuffer.length > contextLines) {
          contextBuffer.shift();
        }
      }
    } else {
      hasChanges = true;

      if (!inChangeBlock) {
        // Don't print context yet - we'll print it only if the hunk isn't filtered
        inChangeBlock = true;
        // Keep contextBuffer for now - we'll use it if we print the hunk
      }

      // Track hunk start line numbers
      if (hunkStartLine1 === null && entry.lineNum1) {
        hunkStartLine1 = entry.lineNum1;
      }
      if (hunkStartLine2 === null && entry.lineNum2) {
        hunkStartLine2 = entry.lineNum2;
      }

      // Collect entries in the current hunk
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
      // Filter individual pairs within the hunk
      const {
        filtered: filteredHunk,
        removeCount: hunkCount1,
        addCount: hunkCount2,
      } = filterHunkEntries(
        currentHunk,
        ignoreAssetIds,
        ignoreUnminifiedRefs,
        ignoreSourceMapUrl,
        ignoreSwappedVariables,
      );

      // Only print if there are any remaining differences after filtering
      if (filteredHunk.length > 0) {
        // Print hunk header
        if (hunkStartLine1 !== null && hunkStartLine2 !== null) {
          printHunkHeader(
            hunkStartLine1,
            hunkCount1,
            hunkStartLine2,
            hunkCount2,
          );
        }

        filteredHunk.forEach(printDiffLine);
        changesPrinted += filteredHunk.length;
        hasPrintedChanges = true;
      }
    }
  }

  if (!hasChanges) {
    console.log(
      `${colors.green}✓ Files are identical (after de-minification)${colors.reset}`,
    );
    return {hunkCount: 0, hasChanges: false};
  } else if (hasPrintedChanges) {
    console.log();
    console.log(
      `${colors.yellow}Total changes: ${changesPrinted}${colors.reset}`,
    );
    return {
      hunkCount: countHunks(
        diff,
        ignoreAssetIds,
        ignoreUnminifiedRefs,
        ignoreSourceMapUrl,
        ignoreSwappedVariables,
      ),
      hasChanges: true,
    };
  } else {
    const ignoredTypes: string[] = [];
    if (ignoreAssetIds) ignoredTypes.push('asset IDs');
    if (ignoreUnminifiedRefs) ignoredTypes.push('unminified refs');
    if (ignoreSourceMapUrl) ignoredTypes.push('source map URLs');
    if (ignoreSwappedVariables) ignoredTypes.push('swapped variables');
    const ignoredText =
      ignoredTypes.length > 0
        ? ` (all differences are ${ignoredTypes.join(' and ')})`
        : '';
    console.log(
      `${colors.green}✓ No significant changes${ignoredText}${colors.reset}`,
    );
    return {hunkCount: 0, hasChanges: false};
  }
}
