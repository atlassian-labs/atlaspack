/* eslint-disable no-console */

/**
 * Diff the results of two different directories
 *
 * 1. Check if they have the same number of files
 * 2. Report the diffreences in files and stop if they differ
 * 3. Check if the files have the same names
 * 4. Check that the file contents is the same
 * 5. Report the diff on the files that differ
 */

import * as fs from 'fs';
import {
  getAllFiles,
  findMatchingFile,
  compareFileContents,
} from './dist-differ-lib.ts';
import type {
  FileInfo,
  MatchingOptions,
} from './dist-differ-lib.ts';

function main() {
  const args = process.argv.slice(2);

  // Parse flags
  const flags = {
    noColor: false,
    forceColor: false,
  };
  
  const filteredArgs = args.filter(arg => {
    if (arg === '--no-color') {
      flags.noColor = true;
      return false;
    }
    if (arg === '--color') {
      flags.forceColor = true;
      return false;
    }
    return true;
  });

  if (filteredArgs.length < 2 || filteredArgs.length > 3) {
    console.error(
      'Usage: node --experimental-strip-types scripts/dist-differ.ts <dir1> <dir2> [sizeThresholdPercent] [options]',
    );
    console.error(
      'Example: node --experimental-strip-types scripts/dist-differ.ts packages/entry-point/dist packages/entry-point/dist-control 5',
    );
    console.error(
      'Parameters:',
    );
    console.error(
      '  <dir1>                   First directory to compare',
    );
    console.error(
      '  <dir2>                   Second directory to compare',
    );
    console.error(
      '  [sizeThresholdPercent]   Size matching tolerance (default: 5)',
    );
    console.error(
      'Options:',
    );
    console.error(
      '  --color                  Force colored output even when not in TTY',
    );
    console.error(
      '  --no-color               Disable colored output',
    );
    console.error(
      'Features:',
    );
    console.error(
      '  • Content hash matching: bundle.abc123.js ↔ bundle.def456.js',
    );
    console.error(
      '  • JavaScript formatting: readable diffs for minified code',
    );
    console.error(
      '  • Identifier filtering: ignores variable/function renames',
    );
    console.error(
      '  • Colored output: red/green highlighting for better readability',
    );
    process.exit(1);
  }

  // Set color preferences
  if (flags.noColor) {
    process.env.NO_COLOR = '1';
  } else if (flags.forceColor) {
    process.env.FORCE_COLOR = '1';
  }

  const [dir1, dir2, thresholdArg] = filteredArgs;
  const options: MatchingOptions = {
    sizeThresholdPercent: thresholdArg ? parseFloat(thresholdArg) : 5,
  };

  if (isNaN(options.sizeThresholdPercent) || options.sizeThresholdPercent < 0) {
    console.error('sizeThresholdPercent must be a non-negative number');
    process.exit(1);
  }

  // Check if directories exist
  if (!fs.existsSync(dir1)) {
    console.error(`Directory does not exist: ${dir1}`);
    process.exit(1);
  }

  if (!fs.existsSync(dir2)) {
    console.error(`Directory does not exist: ${dir2}`);
    process.exit(1);
  }

  console.log(`Comparing directories:`);
  console.log(`  Dir 1: ${dir1}`);
  console.log(`  Dir 2: ${dir2}`);
  console.log(`  Size threshold: ${options.sizeThresholdPercent}%`);
  console.log();

  // Get all files from both directories
  const files1 = getAllFiles(dir1);
  const files2 = getAllFiles(dir2);

  console.log(`Step 1: File counts`);
  console.log(`  Dir 1: ${files1.length} files`);
  console.log(`  Dir 2: ${files2.length} files`);
  console.log();

  console.log(`Step 2: Finding file matches...`);

  const matched = new Set<FileInfo>();
  const comparisons: Array<{
    file1: FileInfo;
    file2: FileInfo;
    matchType: string;
  }> = [];
  const unmatched1: FileInfo[] = [];

  // Find matches for files in dir1
  for (const file1 of files1) {
    const match = findMatchingFile(file1, files2, options);
    if (match && !matched.has(match)) {
      matched.add(match);
      let matchType = 'exact';
      if (file1.relativePath !== match.relativePath) {
        if (file1.normalizedPath === match.normalizedPath) {
          matchType = 'normalized';
        } else {
          matchType = 'size-based';
        }
      }
      comparisons.push({file1, file2: match, matchType});
    } else {
      unmatched1.push(file1);
    }
  }

  // Find unmatched files in dir2
  const unmatched2 = files2.filter((f) => !matched.has(f));

  console.log(`  Found ${comparisons.length} file matches`);
  if (comparisons.length > 0) {
    const exactMatches = comparisons.filter(
      (c) => c.matchType === 'exact',
    ).length;
    const normalizedMatches = comparisons.filter(
      (c) => c.matchType === 'normalized',
    ).length;
    const sizeMatches = comparisons.filter(
      (c) => c.matchType === 'size-based',
    ).length;

    console.log(`    - ${exactMatches} exact path matches`);
    if (normalizedMatches > 0)
      console.log(`    - ${normalizedMatches} normalized path matches`);
    if (sizeMatches > 0) console.log(`    - ${sizeMatches} size-based matches`);
  }

  if (unmatched1.length > 0) {
    console.log(`\n  Files only in ${dir1} (${unmatched1.length}):`);
    unmatched1.forEach((file) =>
      console.log(`    - ${file.relativePath} (${file.size} bytes)`),
    );
  }

  if (unmatched2.length > 0) {
    console.log(`\n  Files only in ${dir2} (${unmatched2.length}):`);
    unmatched2.forEach((file) =>
      console.log(`    - ${file.relativePath} (${file.size} bytes)`),
    );
  }

  console.log(`\nStep 3: Comparing file contents...`);

  let differingFiles = 0;
  let identicalFiles = 0;

  for (const {file1, file2, matchType} of comparisons) {
    const matchInfo = matchType === 'exact' ? '' : ` (${matchType} match)`;

    // Quick size check for non-size-based matches
    if (matchType !== 'size-based' && file1.size !== file2.size) {
      console.log(
        `❌ ${file1.relativePath} -> ${file2.relativePath}${matchInfo}: Size mismatch (${file1.size} vs ${file2.size} bytes)`,
      );
      differingFiles++;
      continue;
    }

    // Content comparison
    const diff = compareFileContents(file1.fullPath, file2.fullPath);
    if (diff !== null) {
      console.log(
        `❌ ${file1.relativePath} -> ${file2.relativePath}${matchInfo}: Content differs`,
      );
      if (matchType !== 'exact') {
        console.log(`   File sizes: ${file1.size} vs ${file2.size} bytes`);
      }
      console.log(`Diff:`);
      console.log(diff);
      console.log(`${'='.repeat(80)}`);
      differingFiles++;
    } else {
      if (matchType !== 'exact') {
        console.log(
          `✅ ${file1.relativePath} -> ${file2.relativePath}${matchInfo}: Content matches`,
        );
      }
      identicalFiles++;
    }
  }

  console.log(`\nSummary:`);
  console.log(`  ✅ ${identicalFiles} files with matching content`);
  console.log(`  ❌ ${differingFiles} files with differing content`);
  console.log(`  📁 ${unmatched1.length} files only in ${dir1}`);
  console.log(`  📁 ${unmatched2.length} files only in ${dir2}`);

  const hasErrors =
    differingFiles > 0 || unmatched1.length > 0 || unmatched2.length > 0;

  if (!hasErrors) {
    console.log(`\n🎉 All matched files are identical!`);
  } else {
    console.log(`\n❌ Found differences between directories`);
    process.exit(1);
  }
}

main();
