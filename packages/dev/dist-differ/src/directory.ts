/* eslint-disable no-console */
import * as fs from 'fs';
import * as path from 'path';
import type {FileInfo} from './types';
import {getColors} from './utils/colors';
import {readAndDeminify} from './utils/deminify';
import {computeDiff} from './diff';
import {countHunks} from './hunk';
import {printDiff} from './print';
import {matchFilesByPrefix} from './match';
import {
  printAmbiguousMatches,
  printFileSummary,
  printComparisonSummary,
} from './report';
import {
  diffToJson,
  generateDirectoryJsonReport,
  writeJsonReportStreaming,
} from './json';
import type {ComparisonContext} from './context';

const colors = getColors();

/**
 * Recursively gets all .js files from a directory
 */
export function getAllFiles(
  dir: string,
  baseDir: string = dir,
  fileList: FileInfo[] = [],
): FileInfo[] {
  // Ensure dir is absolute
  const absDir = path.resolve(dir);
  const absBaseDir = path.resolve(baseDir);
  const entries = fs.readdirSync(absDir, {withFileTypes: true});

  for (const entry of entries) {
    const fullPath = path.resolve(absDir, entry.name);
    const relativePath = path.relative(absBaseDir, fullPath);

    if (entry.isDirectory()) {
      getAllFiles(fullPath, absBaseDir, fileList);
    } else if (entry.isFile()) {
      // Only include .js files, ignore .map files and others
      if (entry.name.endsWith('.js')) {
        const stats = fs.statSync(fullPath);
        fileList.push({
          fullPath,
          relativePath,
          filename: entry.name,
          size: stats.size,
        });
      }
    }
  }

  return fileList;
}

/**
 * Compares two directories of minified files
 */
export function compareDirectories(
  dir1: string,
  dir2: string,
  context: ComparisonContext,
): void {
  // Resolve to absolute paths
  const absDir1 = path.resolve(dir1);
  const absDir2 = path.resolve(dir2);

  if (!context.jsonMode) {
    console.log(`${colors.cyan}=== Comparing directories ===${colors.reset}`);
    console.log(`${colors.yellow}Directory 1:${colors.reset} ${absDir1}`);
    console.log(`${colors.yellow}Directory 2:${colors.reset} ${absDir2}`);
    console.log();
  }

  // Get all files from both directories
  const files1 = getAllFiles(absDir1);
  const files2 = getAllFiles(absDir2);

  // Early exit: file count mismatch
  if (files1.length !== files2.length) {
    if (context.jsonMode) {
      // JSON output for error case
      const report = {
        metadata: {
          dir1: absDir1,
          dir2: absDir2,
          comparisonDate: new Date().toISOString(),
          options: {
            ignoreAssetIds: context.ignoreAssetIds,
            ignoreUnminifiedRefs: context.ignoreUnminifiedRefs,
            ignoreSourceMapUrl: context.ignoreSourceMapUrl,
            ignoreSwappedVariables: context.ignoreSwappedVariables,
          },
        },
        summary: {
          totalHunks: 0,
          meaningfulHunks: 0,
          harmlessHunks: 0,
          identical: false,
          error: 'file_count_mismatch',
          files1Count: files1.length,
          files2Count: files2.length,
        },
      };
      writeJsonReportStreaming(report);
    } else {
      console.log(`${colors.red}✗ File count mismatch${colors.reset}`);
      console.log(`  Directory 1: ${files1.length} files`);
      console.log(`  Directory 2: ${files2.length} files`);
      console.log();

      // Show which files are unique to each directory
      const relativePaths1 = new Set(files1.map((f) => f.relativePath));
      const relativePaths2 = new Set(files2.map((f) => f.relativePath));

      const onlyIn1 = files1.filter((f) => !relativePaths2.has(f.relativePath));
      const onlyIn2 = files2.filter((f) => !relativePaths1.has(f.relativePath));

      if (onlyIn1.length > 0) {
        console.log(
          `${colors.yellow}Files only in directory 1:${colors.reset}`,
        );
        onlyIn1.forEach((f) => console.log(`  ${f.relativePath}`));
      }

      if (onlyIn2.length > 0) {
        console.log(
          `${colors.yellow}Files only in directory 2:${colors.reset}`,
        );
        onlyIn2.forEach((f) => console.log(`  ${f.relativePath}`));
      }
    }

    process.exitCode = 1;
    return;
  }

  // Check if all files have identical names (including hash)
  const relativePaths1 = new Set(files1.map((f) => f.relativePath));
  const relativePaths2 = new Set(files2.map((f) => f.relativePath));

  const allSameNames =
    files1.every((f) => relativePaths2.has(f.relativePath)) &&
    files2.every((f) => relativePaths1.has(f.relativePath));

  if (allSameNames) {
    if (context.jsonMode) {
      const report = {
        metadata: {
          dir1: absDir1,
          dir2: absDir2,
          comparisonDate: new Date().toISOString(),
          options: {
            ignoreAssetIds: context.ignoreAssetIds,
            ignoreUnminifiedRefs: context.ignoreUnminifiedRefs,
            ignoreSourceMapUrl: context.ignoreSourceMapUrl,
            ignoreSwappedVariables: context.ignoreSwappedVariables,
          },
        },
        summary: {
          totalHunks: 0,
          meaningfulHunks: 0,
          harmlessHunks: 0,
          identical: true,
          identicalFiles: files1.length,
          differentFiles: 0,
          totalFiles: files1.length,
        },
      };
      writeJsonReportStreaming(report);
    } else {
      console.log(
        `${colors.green}✓ All files have identical names (including hash)${colors.reset}`,
      );
      console.log(`  Total files: ${files1.length}`);
    }
    return;
  }

  // Match files by prefix
  const {matched, ambiguous} = matchFilesByPrefix(
    files1,
    files2,
    context.sizeThreshold,
  );

  if (context.jsonMode) {
    // Build JSON results
    const fileResults: Array<import('./json').JsonFileResult> = [];

    for (const {file1, file2} of matched) {
      const lines1 = readAndDeminify(file1.fullPath);
      const lines2 = readAndDeminify(file2.fullPath);

      if (!lines1 || !lines2) {
        continue;
      }

      const diff = computeDiff(lines1, lines2);
      const fileResult = diffToJson(
        diff,
        file1.fullPath,
        file2.fullPath,
        context,
      );

      fileResults.push(fileResult);
    }

    const report = generateDirectoryJsonReport(
      matched,
      ambiguous,
      absDir1,
      absDir2,
      context,
      fileResults,
    );

    writeJsonReportStreaming(report);

    if ((report.summary.differentFiles ?? 0) > 0) {
      process.exitCode = 1;
    }
    return;
  }

  // Report ambiguous cases
  if (ambiguous.length > 0) {
    printAmbiguousMatches(ambiguous);
  }

  // Apply minified-diff to matched pairs
  let identicalFiles = 0;
  let differentFiles = 0;
  const filesWithDifferences: Array<{path: string; hunkCount: number}> = []; // For sorting in summary mode

  if (matched.length > 0) {
    if (context.verbose) {
      console.log(
        `${colors.cyan}Comparing ${matched.length} matched file pair(s):${colors.reset}`,
      );
      console.log();
    }

    for (let i = 0; i < matched.length; i++) {
      const {file1, file2} = matched[i];

      // Read and de-minify files
      const lines1 = readAndDeminify(file1.fullPath);
      const lines2 = readAndDeminify(file2.fullPath);

      if (!lines1 || !lines2) {
        continue; // Error already printed
      }

      // Compute diff
      const diff = computeDiff(lines1, lines2);

      if (context.summaryMode) {
        // In summary mode, just count hunks
        const hunkCount = countHunks(
          diff,
          context.ignoreAssetIds,
          context.ignoreUnminifiedRefs,
          context.ignoreSourceMapUrl,
          context.ignoreSwappedVariables,
        );
        const hasChanges = diff.some((e) => e.type !== 'equal');

        if (hasChanges && hunkCount > 0) {
          differentFiles++;
          filesWithDifferences.push({path: file1.relativePath, hunkCount});
        } else {
          identicalFiles++;
          if (context.verbose) {
            console.log(
              `${colors.green}✓${colors.reset} ${file1.relativePath}: identical`,
            );
          }
        }
      } else {
        // In normal mode, print full diff only for files that differ
        const hunkCount = countHunks(
          diff,
          context.ignoreAssetIds,
          context.ignoreUnminifiedRefs,
          context.ignoreSourceMapUrl,
          context.ignoreSwappedVariables,
        );
        const hasChanges = diff.some((e) => e.type !== 'equal');

        if (hasChanges && hunkCount > 0) {
          differentFiles++;
          if (i > 0 && context.verbose) {
            console.log();
          }
          printDiff(
            diff,
            file1.fullPath,
            file2.fullPath,
            3,
            context.ignoreAssetIds,
            context.ignoreUnminifiedRefs,
            context.ignoreSourceMapUrl,
            context.ignoreSwappedVariables,
            false,
          );
        } else {
          identicalFiles++;
          if (context.verbose) {
            if (i > 0) {
              console.log();
            }
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
      }
    }

    // In summary mode, print sorted list of files with differences
    if (context.summaryMode && filesWithDifferences.length > 0) {
      printFileSummary(filesWithDifferences);
    }
  }

  // Print summary
  printComparisonSummary(
    0,
    false,
    identicalFiles,
    differentFiles,
    matched.length,
  );
}
