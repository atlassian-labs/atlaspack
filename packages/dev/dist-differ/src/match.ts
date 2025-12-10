import * as path from 'path';
import * as fs from 'fs';
import type {FileInfo, MatchResult, MatchedPair, AmbiguousMatch} from './types';

/**
 * Formats a file size with commas
 */
export function formatFileSize(bytes: number): string {
  return bytes.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ',');
}

/**
 * Extracts prefix from filename like "async-error-flag-renderer.c272922c.js"
 * Returns "async-error-flag-renderer"
 */
export function extractPrefix(filename: string): string {
  const lastDot = filename.lastIndexOf('.');
  if (lastDot === -1) return filename;

  const beforeLastDot = filename.substring(0, lastDot);
  const secondLastDot = beforeLastDot.lastIndexOf('.');
  if (secondLastDot === -1) return filename;

  return beforeLastDot.substring(0, secondLastDot);
}

/**
 * Groups files by their directory path
 */
function groupByPath(files: FileInfo[]): Record<string, FileInfo[]> {
  const groups: Record<string, FileInfo[]> = {};
  for (const file of files) {
    const dirPath = path.dirname(file.relativePath);
    if (!groups[dirPath]) {
      groups[dirPath] = [];
    }
    groups[dirPath].push(file);
  }
  return groups;
}

/**
 * Matches files from two directories by prefix, with size-based disambiguation
 */
export function matchFilesByPrefix(
  files1: FileInfo[],
  files2: FileInfo[],
  sizeThreshold: number = 0,
): MatchResult {
  const matched: MatchedPair[] = [];
  const ambiguous: AmbiguousMatch[] = [];
  const used1 = new Set<string>();
  const used2 = new Set<string>();

  // Group files by relative path
  const groups1 = groupByPath(files1);
  const groups2 = groupByPath(files2);

  // Get all unique paths
  const allPaths = new Set([...Object.keys(groups1), ...Object.keys(groups2)]);

  for (const dirPath of allPaths) {
    const filesInPath1 = groups1[dirPath] || [];
    const filesInPath2 = groups2[dirPath] || [];

    // First pass: match files with exact same filename
    const filenameMap2 = new Map<string, FileInfo[]>();
    for (const file of filesInPath2) {
      if (!filenameMap2.has(file.filename)) {
        filenameMap2.set(file.filename, []);
      }
      filenameMap2.get(file.filename)!.push(file);
    }

    for (const file1 of filesInPath1) {
      if (used1.has(file1.fullPath)) continue;
      const matchingFiles2 = filenameMap2.get(file1.filename) || [];
      if (matchingFiles2.length > 0) {
        // Found exact match - use first available
        const file2 = matchingFiles2.find((f) => !used2.has(f.fullPath));
        if (file2) {
          matched.push({
            file1,
            file2,
            prefix: extractPrefix(file1.filename),
            dirPath,
          });
          used1.add(file1.fullPath);
          used2.add(file2.fullPath);
        }
      }
    }

    // Second pass: match remaining files by prefix
    const prefixMap1 = new Map<string, FileInfo[]>();
    const prefixMap2 = new Map<string, FileInfo[]>();

    for (const file of filesInPath1) {
      if (used1.has(file.fullPath)) continue;
      const prefix = extractPrefix(file.filename);
      if (!prefixMap1.has(prefix)) {
        prefixMap1.set(prefix, []);
      }
      prefixMap1.get(prefix)!.push(file);
    }

    for (const file of filesInPath2) {
      if (used2.has(file.fullPath)) continue;
      const prefix = extractPrefix(file.filename);
      if (!prefixMap2.has(prefix)) {
        prefixMap2.set(prefix, []);
      }
      prefixMap2.get(prefix)!.push(file);
    }

    // Match files with same prefix
    for (const [prefix, files1WithPrefix] of prefixMap1.entries()) {
      const files2WithPrefix = prefixMap2.get(prefix) || [];

      if (files1WithPrefix.length === 1 && files2WithPrefix.length === 1) {
        // Unambiguous 1:1 match
        const file1 = files1WithPrefix[0];
        const file2 = files2WithPrefix[0];
        matched.push({file1, file2, prefix, dirPath});
        used1.add(file1.fullPath);
        used2.add(file2.fullPath);
      } else if (files1WithPrefix.length > 0 || files2WithPrefix.length > 0) {
        // Check if we can match by exact filename within the prefix group
        const remaining1 = files1WithPrefix.filter(
          (f) => !used1.has(f.fullPath),
        );
        const remaining2 = files2WithPrefix.filter(
          (f) => !used2.has(f.fullPath),
        );

        // Try to match exact filenames within this prefix group
        const filenameMap2InPrefix = new Map<string, FileInfo[]>();
        for (const file of remaining2) {
          if (!filenameMap2InPrefix.has(file.filename)) {
            filenameMap2InPrefix.set(file.filename, []);
          }
          filenameMap2InPrefix.get(file.filename)!.push(file);
        }

        const stillUnmatched1: FileInfo[] = [];
        for (const file1 of remaining1) {
          const matchingFiles2 = filenameMap2InPrefix.get(file1.filename) || [];
          if (matchingFiles2.length > 0) {
            const file2 = matchingFiles2.find((f) => !used2.has(f.fullPath));
            if (file2) {
              matched.push({file1, file2, prefix, dirPath});
              used1.add(file1.fullPath);
              used2.add(file2.fullPath);
            } else {
              stillUnmatched1.push(file1);
            }
          } else {
            stillUnmatched1.push(file1);
          }
        }

        const stillUnmatched2 = remaining2.filter(
          (f) => !used2.has(f.fullPath),
        );

        // Try to resolve ambiguity by matching file sizes
        const resolvedBySize1: FileInfo[] = [];
        const resolvedBySize2: FileInfo[] = [];

        if (stillUnmatched1.length > 0 && stillUnmatched2.length > 0) {
          // Group files by size
          const sizeMap1 = new Map<number, FileInfo[]>();
          const sizeMap2 = new Map<number, FileInfo[]>();

          for (const file of stillUnmatched1) {
            if (!sizeMap1.has(file.size)) {
              sizeMap1.set(file.size, []);
            }
            sizeMap1.get(file.size)!.push(file);
          }

          for (const file of stillUnmatched2) {
            if (!sizeMap2.has(file.size)) {
              sizeMap2.set(file.size, []);
            }
            sizeMap2.get(file.size)!.push(file);
          }

          // Match files with unique 1:1 size matches
          for (const [size, files1WithSize] of sizeMap1.entries()) {
            const files2WithSize = sizeMap2.get(size) || [];

            // Only match if both sets have exactly one file with this size (unique 1:1 match)
            if (files1WithSize.length === 1 && files2WithSize.length === 1) {
              const file1 = files1WithSize[0];
              const file2 = files2WithSize[0];

              // Make sure these files haven't been used already
              if (!used1.has(file1.fullPath) && !used2.has(file2.fullPath)) {
                matched.push({file1, file2, prefix, dirPath});
                used1.add(file1.fullPath);
                used2.add(file2.fullPath);
                resolvedBySize1.push(file1);
                resolvedBySize2.push(file2);
              }
            }
          }
        }

        // Remaining unmatched files after exact size-based resolution
        let stillUnmatchedAfterExact1 = stillUnmatched1.filter(
          (f) => !resolvedBySize1.includes(f),
        );
        let stillUnmatchedAfterExact2 = stillUnmatched2.filter(
          (f) => !resolvedBySize2.includes(f),
        );

        // Try "close enough" size matching if threshold is set and there are still unmatched files
        const resolvedByCloseSize1: FileInfo[] = [];
        const resolvedByCloseSize2: FileInfo[] = [];

        if (
          sizeThreshold > 0 &&
          stillUnmatchedAfterExact1.length > 0 &&
          stillUnmatchedAfterExact2.length > 0
        ) {
          // Calculate all possible pairs with their size differences (as percentage)
          const candidatePairs: Array<{
            file1: FileInfo;
            file2: FileInfo;
            percentDiff: number;
            sizeDiff: number;
          }> = [];
          for (const file1 of stillUnmatchedAfterExact1) {
            for (const file2 of stillUnmatchedAfterExact2) {
              const largerSize = Math.max(file1.size, file2.size);
              const sizeDiff = Math.abs(file1.size - file2.size);
              const percentDiff = largerSize > 0 ? sizeDiff / largerSize : 1;

              if (percentDiff <= sizeThreshold) {
                candidatePairs.push({
                  file1,
                  file2,
                  percentDiff,
                  sizeDiff,
                });
              }
            }
          }

          // Sort by size difference (smallest first) for best matches
          // Add deterministic tie-breaking for consistent results
          candidatePairs.sort((a, b) => {
            // First by percentage difference
            if (a.percentDiff !== b.percentDiff) {
              return a.percentDiff - b.percentDiff;
            }
            // Then by absolute difference
            if (a.sizeDiff !== b.sizeDiff) {
              return a.sizeDiff - b.sizeDiff;
            }
            // Deterministic tie-breaker: sort by file paths for consistent ordering
            const pathCompare = a.file1.relativePath.localeCompare(
              b.file1.relativePath,
            );
            if (pathCompare !== 0) {
              return pathCompare;
            }
            return a.file2.relativePath.localeCompare(b.file2.relativePath);
          });

          // Greedily match pairs, ensuring each file is only matched once
          const matched1 = new Set<string>();
          const matched2 = new Set<string>();

          for (const pair of candidatePairs) {
            if (
              !matched1.has(pair.file1.fullPath) &&
              !matched2.has(pair.file2.fullPath)
            ) {
              // Make sure these files haven't been used already
              if (
                !used1.has(pair.file1.fullPath) &&
                !used2.has(pair.file2.fullPath)
              ) {
                matched.push({
                  file1: pair.file1,
                  file2: pair.file2,
                  prefix,
                  dirPath,
                });
                used1.add(pair.file1.fullPath);
                used2.add(pair.file2.fullPath);
                matched1.add(pair.file1.fullPath);
                matched2.add(pair.file2.fullPath);
                resolvedByCloseSize1.push(pair.file1);
                resolvedByCloseSize2.push(pair.file2);
              }
            }
          }
        }

        // Final remaining unmatched files after all size-based resolution
        const finalUnmatched1 = stillUnmatchedAfterExact1.filter(
          (f) => !resolvedByCloseSize1.includes(f),
        );
        const finalUnmatched2 = stillUnmatchedAfterExact2.filter(
          (f) => !resolvedByCloseSize2.includes(f),
        );

        // If there are still unmatched files, mark as ambiguous
        if (finalUnmatched1.length > 0 || finalUnmatched2.length > 0) {
          ambiguous.push({
            prefix,
            dirPath,
            files1: finalUnmatched1,
            files2: finalUnmatched2,
          });
        }
      }
    }

    // Check for files in dir2 that don't have matches in dir1
    for (const [prefix, files2WithPrefix] of prefixMap2.entries()) {
      if (!prefixMap1.has(prefix)) {
        const remaining2 = files2WithPrefix.filter(
          (f) => !used2.has(f.fullPath),
        );
        if (remaining2.length > 0) {
          ambiguous.push({
            prefix,
            dirPath,
            files1: [],
            files2: remaining2,
          });
        }
      }
    }
  }

  return {matched, ambiguous};
}
