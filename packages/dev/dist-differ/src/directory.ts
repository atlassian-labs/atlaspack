/* eslint-disable no-console */
import * as fs from 'fs';
import * as path from 'path';
import type {FileInfo} from './types';
import {getColors} from './utils/colors';
import {readAndDeminify} from './utils/deminify';
import {computeDiff} from './diff';
import {countHunks} from './hunk';
import {printDiff} from './print';
import {matchFilesByPrefix, formatFileSize, extractPrefix} from './match';

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
  ignoreAssetIds: boolean,
  ignoreUnminifiedRefs: boolean,
  ignoreSourceMapUrl: boolean,
  ignoreSwappedVariables: boolean,
  summaryMode: boolean = false,
  verbose: boolean = false,
  sizeThreshold: number = 0.01,
): void {
  // Resolve to absolute paths
  const absDir1 = path.resolve(dir1);
  const absDir2 = path.resolve(dir2);

  console.log(`${colors.cyan}=== Comparing directories ===${colors.reset}`);
  console.log(`${colors.yellow}Directory 1:${colors.reset} ${absDir1}`);
  console.log(`${colors.yellow}Directory 2:${colors.reset} ${absDir2}`);
  console.log();

  // Get all files from both directories
  const files1 = getAllFiles(absDir1);
  const files2 = getAllFiles(absDir2);

  // Early exit: file count mismatch
  if (files1.length !== files2.length) {
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
      console.log(`${colors.yellow}Files only in directory 1:${colors.reset}`);
      onlyIn1.forEach((f) => console.log(`  ${f.relativePath}`));
    }

    if (onlyIn2.length > 0) {
      console.log(`${colors.yellow}Files only in directory 2:${colors.reset}`);
      onlyIn2.forEach((f) => console.log(`  ${f.relativePath}`));
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
    console.log(
      `${colors.green}✓ All files have identical names (including hash)${colors.reset}`,
    );
    console.log(`  Total files: ${files1.length}`);
    return;
  }

  // Match files by prefix
  const {matched, ambiguous} = matchFilesByPrefix(
    files1,
    files2,
    sizeThreshold,
  );

  // Report ambiguous cases
  if (ambiguous.length > 0) {
    console.log(`${colors.yellow}⚠ Ambiguous file matches:${colors.reset}`);
    for (const amb of ambiguous) {
      if (amb.files1.length === 0 && amb.files2.length > 0) {
        console.log(`  Prefix "${amb.prefix}" in ${amb.dirPath}:`);
        console.log(`    Only in directory 2:`);
        amb.files2.forEach((f) =>
          console.log(
            `      ${f.relativePath} (${formatFileSize(f.size)} bytes)`,
          ),
        );
      } else if (amb.files1.length > 0 && amb.files2.length === 0) {
        console.log(`  Prefix "${amb.prefix}" in ${amb.dirPath}:`);
        console.log(`    Only in directory 1:`);
        amb.files1.forEach((f) =>
          console.log(
            `      ${f.relativePath} (${formatFileSize(f.size)} bytes)`,
          ),
        );
      } else {
        console.log(`  Prefix "${amb.prefix}" in ${amb.dirPath}:`);
        console.log(`    Directory 1 (${amb.files1.length} file(s)):`);
        amb.files1.forEach((f) =>
          console.log(
            `      ${f.relativePath} (${formatFileSize(f.size)} bytes)`,
          ),
        );
        console.log(`    Directory 2 (${amb.files2.length} file(s)):`);
        amb.files2.forEach((f) =>
          console.log(
            `      ${f.relativePath} (${formatFileSize(f.size)} bytes)`,
          ),
        );
      }
    }
    console.log();
  }

  // Apply minified-diff to matched pairs
  let identicalFiles = 0;
  let differentFiles = 0;
  const filesWithDifferences: Array<{path: string; hunkCount: number}> = []; // For sorting in summary mode

  if (matched.length > 0) {
    if (verbose) {
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

      if (summaryMode) {
        // In summary mode, just count hunks
        const hunkCount = countHunks(
          diff,
          ignoreAssetIds,
          ignoreUnminifiedRefs,
          ignoreSourceMapUrl,
          ignoreSwappedVariables,
        );
        const hasChanges = diff.some((e) => e.type !== 'equal');

        if (hasChanges && hunkCount > 0) {
          differentFiles++;
          filesWithDifferences.push({path: file1.relativePath, hunkCount});
        } else {
          identicalFiles++;
          if (verbose) {
            console.log(
              `${colors.green}✓${colors.reset} ${file1.relativePath}: identical`,
            );
          }
        }
      } else {
        // In normal mode, print full diff only for files that differ
        const hunkCount = countHunks(
          diff,
          ignoreAssetIds,
          ignoreUnminifiedRefs,
          ignoreSourceMapUrl,
          ignoreSwappedVariables,
        );
        const hasChanges = diff.some((e) => e.type !== 'equal');

        if (hasChanges && hunkCount > 0) {
          differentFiles++;
          if (i > 0 && verbose) {
            console.log();
          }
          printDiff(
            diff,
            file1.fullPath,
            file2.fullPath,
            3,
            ignoreAssetIds,
            ignoreUnminifiedRefs,
            ignoreSourceMapUrl,
            ignoreSwappedVariables,
            false,
          );
        } else {
          identicalFiles++;
          if (verbose) {
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
    if (summaryMode && filesWithDifferences.length > 0) {
      // Sort by hunk count (descending - most hunks first)
      filesWithDifferences.sort((a, b) => b.hunkCount - a.hunkCount);

      // Print the sorted list
      for (const fileInfo of filesWithDifferences) {
        console.log(
          `${colors.yellow}${fileInfo.path}${colors.reset}: ${fileInfo.hunkCount} hunk(s) differ`,
        );
      }
    }
  }

  // Print summary
  console.log();
  console.log(`${colors.cyan}=== Summary ===${colors.reset}`);
  console.log(
    `  ${colors.green}Identical files: ${identicalFiles}${colors.reset}`,
  );
  console.log(
    `  ${colors.yellow}Different files: ${differentFiles}${colors.reset}`,
  );
  console.log(
    `  ${colors.cyan}Total files compared: ${matched.length}${colors.reset}`,
  );
}
