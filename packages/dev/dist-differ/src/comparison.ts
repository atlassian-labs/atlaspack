/* eslint-disable no-console */
import * as path from 'path';
import {readAndDeminify} from './utils/deminify';
import {computeDiff} from './diff';
import {countHunks} from './hunk';
import {printDiff} from './print';
import {getAllFiles} from './directory';
import {matchFilesByPrefix, extractPrefix} from './match';
import type {FileInfo, MatchedPair} from './types';
import type {CliOptions} from './cli';
import {
  printAmbiguousMatches,
  printFileSummary,
  printComparisonSummary,
} from './report';
import {getColors} from './utils/colors';

/**
 * Compares two files and prints the diff
 */
export function compareFiles(
  file1: string,
  file2: string,
  options: CliOptions,
): void {
  try {
    const absFile1 = path.resolve(file1);
    const absFile2 = path.resolve(file2);

    const lines1 = readAndDeminify(absFile1);
    const lines2 = readAndDeminify(absFile2);

    if (!lines1 || !lines2) {
      return; // Error already printed
    }

    // Compute and print diff
    const diff = computeDiff(lines1, lines2);
    const result = printDiff(
      diff,
      absFile1,
      absFile2,
      3,
      options.ignoreAssetIds,
      options.ignoreUnminifiedRefs,
      options.ignoreSourceMapUrl,
      options.ignoreSwappedVariables,
      options.summaryMode,
    );

    // Show summary for file comparison
    if (options.summaryMode) {
      printComparisonSummary(result.hunkCount, result.hasChanges);
    }
  } catch (error) {
    console.error(
      `Unexpected error comparing files: ${error instanceof Error ? error.message : String(error)}`,
    );
    if (error instanceof Error && error.stack) {
      console.error(error.stack);
    }
    process.exitCode = 1;
  }
}

/**
 * Compares files by prefix when paths don't exist as files
 */
export function compareFilesByPrefix(
  prefix1: string,
  prefix2: string,
  dir1: string,
  dir2: string,
  options: CliOptions,
): void {
  // Search for files matching the prefix in both directories
  const files1 = getAllFiles(dir1).filter((f) => {
    const filePrefix = extractPrefix(f.filename);
    return filePrefix === prefix1 && f.filename.endsWith('.js');
  });

  const files2 = getAllFiles(dir2).filter((f) => {
    const filePrefix = extractPrefix(f.filename);
    return filePrefix === prefix2 && f.filename.endsWith('.js');
  });

  if (files1.length === 0) {
    console.error(
      `Error: No files found matching prefix "${prefix1}" in ${dir1}`,
    );
    process.exitCode = 1;
    return;
  }

  if (files2.length === 0) {
    console.error(
      `Error: No files found matching prefix "${prefix2}" in ${dir2}`,
    );
    process.exitCode = 1;
    return;
  }

  // Use disambiguation logic when multiple files match
  if (files1.length > 1 || files2.length > 1) {
    compareMultipleFilesByPrefix(prefix1, prefix2, files1, files2, options);
    return;
  }

  // Exactly one match in each directory - compare them
  compareSingleFilePair(files1[0], files2[0], prefix1, prefix2, options);
}

/**
 * Compares a single matched file pair
 */
function compareSingleFilePair(
  file1: FileInfo,
  file2: FileInfo,
  prefix1: string,
  prefix2: string,
  options: CliOptions,
): void {
  try {
    const colors = getColors();

    console.log(
      `${colors.cyan}=== Matching files by prefix ===${colors.reset}`,
    );
    console.log(
      `${colors.yellow}Prefix 1:${colors.reset} ${prefix1} -> ${file1.relativePath}`,
    );
    console.log(
      `${colors.yellow}Prefix 2:${colors.reset} ${prefix2} -> ${file2.relativePath}`,
    );
    console.log();

    // Read and de-minify files
    const lines1 = readAndDeminify(file1.fullPath);
    const lines2 = readAndDeminify(file2.fullPath);

    if (!lines1 || !lines2) {
      return; // Error already printed
    }

    // Compute and print diff
    const diff = computeDiff(lines1, lines2);
    const result = printDiff(
      diff,
      file1.fullPath,
      file2.fullPath,
      3,
      options.ignoreAssetIds,
      options.ignoreUnminifiedRefs,
      options.ignoreSourceMapUrl,
      options.ignoreSwappedVariables,
      options.summaryMode,
    );

    // Show summary for file comparison
    if (options.summaryMode) {
      printComparisonSummary(result.hunkCount, result.hasChanges);
    }
  } catch (error) {
    console.error(
      `Unexpected error comparing files ${file1.relativePath} and ${file2.relativePath}: ${error instanceof Error ? error.message : String(error)}`,
    );
    if (error instanceof Error && error.stack) {
      console.error(error.stack);
    }
    process.exitCode = 1;
  }
}

/**
 * Compares multiple file pairs when there are multiple matches
 */
function compareMultipleFilesByPrefix(
  prefix1: string,
  prefix2: string,
  files1: FileInfo[],
  files2: FileInfo[],
  options: CliOptions,
): void {
  const colors = getColors();

  console.log(`${colors.cyan}=== Matching files by prefix ===${colors.reset}`);
  console.log(
    `${colors.yellow}Prefix 1:${colors.reset} ${prefix1} (${files1.length} file(s) found)`,
  );
  console.log(
    `${colors.yellow}Prefix 2:${colors.reset} ${prefix2} (${files2.length} file(s) found)`,
  );
  console.log();

  // Use disambiguation logic to match files
  const {matched, ambiguous} = matchFilesByPrefix(
    files1,
    files2,
    options.sizeThreshold,
  );

  // Report ambiguous cases
  if (ambiguous.length > 0) {
    printAmbiguousMatches(ambiguous);
  }

  // Compare all matched pairs
  if (matched.length === 0) {
    console.log(`${colors.red}✗ No files could be matched${colors.reset}`);
    process.exitCode = 1;
    return;
  }

  const comparisonResults = compareMatchedPairs(matched, options);

  // In summary mode, print sorted list of files with differences
  if (
    options.summaryMode &&
    comparisonResults.filesWithDifferences.length > 0
  ) {
    printFileSummary(comparisonResults.filesWithDifferences);
  }

  // Show summary
  printComparisonSummary(
    comparisonResults.totalHunks,
    comparisonResults.hasAnyChanges,
    comparisonResults.identicalFiles,
    comparisonResults.differentFiles,
    matched.length,
  );

  if (comparisonResults.differentFiles > 0) {
    process.exitCode = 1;
  }
}

/**
 * Compares all matched file pairs and returns statistics
 */
function compareMatchedPairs(
  matched: MatchedPair[],
  options: CliOptions,
): {
  identicalFiles: number;
  differentFiles: number;
  filesWithDifferences: Array<{path: string; hunkCount: number}>;
  totalHunks: number;
  hasAnyChanges: boolean;
} {
  let identicalFiles = 0;
  let differentFiles = 0;
  const filesWithDifferences: Array<{path: string; hunkCount: number}> = [];
  let totalHunks = 0;
  let hasAnyChanges = false;

  for (let i = 0; i < matched.length; i++) {
    const {file1, file2} = matched[i];

    try {
      // Read and de-minify files
      const lines1 = readAndDeminify(file1.fullPath);
      const lines2 = readAndDeminify(file2.fullPath);

      if (!lines1 || !lines2) {
        continue; // Error already printed
      }

      // Compute diff
      const diff = computeDiff(lines1, lines2);
      const hunkCount = countHunks(
        diff,
        options.ignoreAssetIds,
        options.ignoreUnminifiedRefs,
        options.ignoreSourceMapUrl,
        options.ignoreSwappedVariables,
      );
      const hasChanges = diff.some((e) => e.type !== 'equal');

      if (options.summaryMode) {
        // In summary mode, just count hunks
        if (hasChanges && hunkCount > 0) {
          differentFiles++;
          filesWithDifferences.push({path: file1.relativePath, hunkCount});
          totalHunks += hunkCount;
          hasAnyChanges = true;
        } else {
          identicalFiles++;
        }
      } else {
        // In normal mode, print full diff only for files that differ
        if (hasChanges && hunkCount > 0) {
          differentFiles++;
          totalHunks += hunkCount;
          hasAnyChanges = true;
          if (i > 0) {
            console.log();
          }
          printDiff(
            diff,
            file1.fullPath,
            file2.fullPath,
            3,
            options.ignoreAssetIds,
            options.ignoreUnminifiedRefs,
            options.ignoreSourceMapUrl,
            options.ignoreSwappedVariables,
            false,
          );
        } else {
          identicalFiles++;
          if (i > 0) {
            console.log();
          }
          const colors = getColors();
          console.log(`${colors.cyan}=== Comparing files ===${colors.reset}`);
          console.log(
            `${colors.yellow}File 1:${colors.reset} ${file1.fullPath}`,
          );
          console.log(
            `${colors.yellow}File 2:${colors.reset} ${file2.fullPath}`,
          );
          console.log();
          console.log(
            `${colors.green}✓ Files are identical (after de-minification)${colors.reset}`,
          );
        }
      }
    } catch (error) {
      console.error(
        `Unexpected error comparing files ${file1.relativePath} and ${file2.relativePath}: ${error instanceof Error ? error.message : String(error)}`,
      );
      if (error instanceof Error && error.stack) {
        console.error(error.stack);
      }
      process.exitCode = 1;
      // Continue with next file pair
      continue;
    }
  }

  return {
    identicalFiles,
    differentFiles,
    filesWithDifferences,
    totalHunks,
    hasAnyChanges,
  };
}
