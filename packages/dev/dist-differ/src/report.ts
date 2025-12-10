/* eslint-disable no-console */
import {getColors} from './utils/colors';
import {formatFileSize} from './match';
import type {AmbiguousMatch} from './types';

const colors = getColors();

/**
 * Prints summary of comparison results
 */
export function printComparisonSummary(
  hunkCount: number,
  hasChanges: boolean,
  identicalFiles?: number,
  differentFiles?: number,
  totalFiles?: number,
): void {
  console.log();
  console.log(`${colors.cyan}=== Summary ===${colors.reset}`);

  if (identicalFiles !== undefined && differentFiles !== undefined) {
    // Directory comparison summary
    console.log(
      `  ${colors.green}Identical files: ${identicalFiles}${colors.reset}`,
    );
    console.log(
      `  ${colors.yellow}Different files: ${differentFiles}${colors.reset}`,
    );
    if (totalFiles !== undefined) {
      console.log(
        `  ${colors.cyan}Total files compared: ${totalFiles}${colors.reset}`,
      );
    }
  } else {
    // Single file comparison summary
    if (hasChanges && hunkCount > 0) {
      console.log(
        `  ${colors.yellow}Different: ${hunkCount} hunk(s) differ${colors.reset}`,
      );
    } else {
      console.log(`  ${colors.green}Identical${colors.reset}`);
    }
  }
}

/**
 * Prints a sorted list of files with differences (for summary mode)
 */
export function printFileSummary(
  filesWithDifferences: Array<{path: string; hunkCount: number}>,
): void {
  // Sort by hunk count (descending - most hunks first)
  filesWithDifferences.sort((a, b) => b.hunkCount - a.hunkCount);

  // Print the sorted list
  for (const fileInfo of filesWithDifferences) {
    console.log(
      `${colors.yellow}${fileInfo.path}${colors.reset}: ${fileInfo.hunkCount} hunk(s) differ`,
    );
  }
}

/**
 * Prints ambiguous file matches
 */
export function printAmbiguousMatches(ambiguous: AmbiguousMatch[]): void {
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
