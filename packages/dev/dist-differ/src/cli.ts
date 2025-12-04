/* eslint-disable no-console */
import * as fs from 'fs';
import * as path from 'path';
import {getColors} from './utils/colors';
import {readAndDeminify} from './utils/deminify';
import {computeDiff} from './diff';
import {countHunks} from './hunk';
import {printDiff} from './print';
import {compareDirectories, getAllFiles} from './directory';
import {matchFilesByPrefix, formatFileSize, extractPrefix} from './match';

const colors = getColors();

export interface CliOptions {
  ignoreAssetIds: boolean;
  ignoreUnminifiedRefs: boolean;
  ignoreSourceMapUrl: boolean;
  ignoreSwappedVariables: boolean;
  summaryMode: boolean;
  verbose: boolean;
  sizeThreshold: number;
}

export function parseArgs(args: string[]): {
  options: CliOptions;
  files: string[];
  error?: string;
} {
  const options: CliOptions = {
    ignoreAssetIds: false,
    ignoreUnminifiedRefs: false,
    ignoreSourceMapUrl: false,
    ignoreSwappedVariables: false,
    summaryMode: false,
    verbose: false,
    sizeThreshold: 0.01, // Default 1%
  };
  const files: string[] = [];

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === '--ignore-all') {
      options.ignoreAssetIds = true;
      options.ignoreUnminifiedRefs = true;
      options.ignoreSourceMapUrl = true;
      options.ignoreSwappedVariables = true;
    } else if (arg === '--ignore-asset-ids') {
      options.ignoreAssetIds = true;
    } else if (arg === '--ignore-unminified-refs') {
      options.ignoreUnminifiedRefs = true;
    } else if (arg === '--ignore-source-map-url') {
      options.ignoreSourceMapUrl = true;
    } else if (arg === '--ignore-swapped-variables') {
      options.ignoreSwappedVariables = true;
    } else if (arg === '--summary') {
      options.summaryMode = true;
    } else if (arg === '--verbose') {
      options.verbose = true;
    } else if (arg === '--disambiguation-size-threshold') {
      if (i + 1 >= args.length) {
        return {
          options,
          files,
          error: 'Error: --disambiguation-size-threshold requires a value',
        };
      }
      const thresholdValue = parseFloat(args[i + 1]);
      if (isNaN(thresholdValue) || thresholdValue < 0 || thresholdValue > 1) {
        return {
          options,
          files,
          error:
            'Error: --disambiguation-size-threshold must be a number between 0 and 1',
        };
      }
      options.sizeThreshold = thresholdValue;
      i++; // Skip the next argument as it's the value
    } else if (!arg.startsWith('--')) {
      files.push(arg);
    } else {
      return {options, files, error: `Error: Unknown flag: ${arg}`};
    }
  }

  return {options, files};
}

export function printUsage(): void {
  console.error(
    'Usage: node dist-differ.ts [OPTIONS] <file1|dir1> <file2|dir2>',
  );
  console.error('');
  console.error(
    'Compares two minified files or directories by splitting on semicolons and commas and displaying a diff.',
  );
  console.error(
    'When comparing directories, files are matched by prefix (name before hash).',
  );
  console.error('');
  console.error('Options:');
  console.error(
    '  --ignore-all                          Skip all ignorable differences (equivalent to all --ignore-* flags)',
  );
  console.error(
    '  --ignore-asset-ids                    Skip hunks where the only differences are asset IDs',
  );
  console.error(
    '  --ignore-unminified-refs             Skip hunks where the only differences are unminified refs',
  );
  console.error(
    '                                        (e.g., $e3f4b1abd74dab96$exports, $00042ef5514babaf$var$...)',
  );
  console.error(
    '  --ignore-source-map-url               Skip hunks where the only differences are source map URLs',
  );
  console.error(
    '  --ignore-swapped-variables            Skip hunks where the only differences are swapped variable names',
  );
  console.error(
    '                                        (e.g., t vs a where functionality is identical)',
  );
  console.error(
    '  --summary                            Show only hunk counts for changed files (directory mode only)',
  );
  console.error(
    '  --verbose                            Show all file matches, not just mismatches (directory mode only)',
  );
  console.error(
    '  --disambiguation-size-threshold <val> Threshold for matching files by "close enough" sizes',
  );
  console.error(
    '                                        (default: 0.01 = 1%, range: 0-1)',
  );
  console.error('');
  console.error('Examples:');
  console.error('  node dist-differ.ts file1.js file2.js');
  console.error('  node dist-differ.ts dir1/ dir2/');
  console.error(
    '  node dist-differ.ts --ignore-asset-ids --summary dir1/ dir2/',
  );
}

/**
 * Handles prefix-based file matching when paths don't exist as files
 */
function handlePrefixMatching(
  file1: string,
  file2: string,
  options: CliOptions,
): void {
  // Resolve to absolute paths first
  const absFile1 = path.resolve(file1);
  const absFile2 = path.resolve(file2);

  // Extract parent directory and prefix from each path
  const dir1 = path.dirname(absFile1);
  const dir2 = path.dirname(absFile2);
  const prefix1 = path.basename(absFile1);
  const prefix2 = path.basename(absFile2);

  // Check if parent directories exist
  if (!fs.existsSync(dir1) || !fs.statSync(dir1).isDirectory()) {
    console.error(`Error: Path not found: ${absFile1}`);
    process.exitCode = 1;
    return;
  }

  if (!fs.existsSync(dir2) || !fs.statSync(dir2).isDirectory()) {
    console.error(`Error: Path not found: ${absFile2}`);
    process.exitCode = 1;
    return;
  }

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
    console.log(
      `${colors.cyan}=== Matching files by prefix ===${colors.reset}`,
    );
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

    // Compare all matched pairs
    if (matched.length === 0) {
      console.log(`${colors.red}✗ No files could be matched${colors.reset}`);
      process.exitCode = 1;
      return;
    }

    let identicalFiles = 0;
    let differentFiles = 0;
    const filesWithDifferences: Array<{path: string; hunkCount: number}> = [];

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

      if (options.summaryMode) {
        // In summary mode, just count hunks
        const hunkCount = countHunks(
          diff,
          options.ignoreAssetIds,
          options.ignoreUnminifiedRefs,
          options.ignoreSourceMapUrl,
          options.ignoreSwappedVariables,
        );
        const hasChanges = diff.some((e) => e.type !== 'equal');

        if (hasChanges && hunkCount > 0) {
          differentFiles++;
          filesWithDifferences.push({path: file1.relativePath, hunkCount});
        } else {
          identicalFiles++;
        }
      } else {
        // In normal mode, print full diff only for files that differ
        const hunkCount = countHunks(
          diff,
          options.ignoreAssetIds,
          options.ignoreUnminifiedRefs,
          options.ignoreSourceMapUrl,
          options.ignoreSwappedVariables,
        );
        const hasChanges = diff.some((e) => e.type !== 'equal');

        if (hasChanges && hunkCount > 0) {
          differentFiles++;
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

    // In summary mode, print sorted list of files with differences
    if (options.summaryMode && filesWithDifferences.length > 0) {
      // Sort by hunk count (descending - most hunks first)
      filesWithDifferences.sort((a, b) => b.hunkCount - a.hunkCount);

      // Print the sorted list
      for (const fileInfo of filesWithDifferences) {
        console.log(
          `${colors.yellow}${fileInfo.path}${colors.reset}: ${fileInfo.hunkCount} hunk(s) differ`,
        );
      }
    }

    // Show summary
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

    if (differentFiles > 0) {
      process.exitCode = 1;
    }

    return;
  }

  // Exactly one match in each directory - compare them
  const matchedFile1 = files1[0].fullPath;
  const matchedFile2 = files2[0].fullPath;

  console.log(`${colors.cyan}=== Matching files by prefix ===${colors.reset}`);
  console.log(
    `${colors.yellow}Prefix 1:${colors.reset} ${prefix1} -> ${files1[0].relativePath}`,
  );
  console.log(
    `${colors.yellow}Prefix 2:${colors.reset} ${prefix2} -> ${files2[0].relativePath}`,
  );
  console.log();

  // Read and de-minify files
  const lines1 = readAndDeminify(matchedFile1);
  const lines2 = readAndDeminify(matchedFile2);

  if (!lines1 || !lines2) {
    return; // Error already printed
  }

  // Compute and print diff
  const diff = computeDiff(lines1, lines2);
  const result = printDiff(
    diff,
    files1[0].fullPath,
    files2[0].fullPath,
    3,
    options.ignoreAssetIds,
    options.ignoreUnminifiedRefs,
    options.ignoreSourceMapUrl,
    options.ignoreSwappedVariables,
    options.summaryMode,
  );

  // Show summary for file comparison too
  if (options.summaryMode) {
    console.log();
    console.log(`${colors.cyan}=== Summary ===${colors.reset}`);
    if (result.hasChanges && result.hunkCount > 0) {
      console.log(
        `  ${colors.yellow}Different: ${result.hunkCount} hunk(s) differ${colors.reset}`,
      );
    } else {
      console.log(`  ${colors.green}Identical${colors.reset}`);
    }
  }
}

export function main(): void {
  const args = process.argv.slice(2);

  const {options, files, error} = parseArgs(args);

  if (error) {
    console.error(error);
    process.exitCode = 1;
    return;
  }

  if (files.length !== 2) {
    printUsage();
    process.exitCode = 1;
    return;
  }

  const [file1, file2] = files;

  // Check if paths exist
  const exists1 = fs.existsSync(file1);
  const exists2 = fs.existsSync(file2);

  // If paths don't exist, try to treat them as prefix patterns
  if (!exists1 || !exists2) {
    handlePrefixMatching(file1, file2, options);
    return;
  }

  // Check if both are directories
  const stat1 = fs.statSync(file1);
  const stat2 = fs.statSync(file2);

  if (stat1.isDirectory() && stat2.isDirectory()) {
    // Compare directories (paths will be resolved to absolute inside compareDirectories)
    compareDirectories(
      file1,
      file2,
      options.ignoreAssetIds,
      options.ignoreUnminifiedRefs,
      options.ignoreSourceMapUrl,
      options.ignoreSwappedVariables,
      options.summaryMode,
      options.verbose,
      options.sizeThreshold,
    );
    return;
  } else if (stat1.isDirectory() || stat2.isDirectory()) {
    console.error('Error: Cannot compare a directory with a file');
    console.error('  Both arguments must be either files or directories');
    process.exitCode = 1;
    return;
  }

  // Both are files - compare them
  // Resolve to absolute paths
  const absFile1 = path.resolve(file1);
  const absFile2 = path.resolve(file2);

  const lines1 = readAndDeminify(absFile1);
  const lines2 = readAndDeminify(absFile2);

  if (!lines1 || !lines2) {
    return; // Error already printed
  }

  // Compute and print diff
  const diff = computeDiff(lines1, lines2);
  printDiff(
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

  // Show summary for file comparison too
  if (options.summaryMode) {
    const hunkCount = countHunks(
      diff,
      options.ignoreAssetIds,
      options.ignoreUnminifiedRefs,
      options.ignoreSourceMapUrl,
    );
    const hasChanges = diff.some((e) => e.type !== 'equal');
    console.log();
    console.log(`${colors.cyan}=== Summary ===${colors.reset}`);
    if (hasChanges && hunkCount > 0) {
      console.log(
        `  ${colors.yellow}Different: ${hunkCount} hunk(s) differ${colors.reset}`,
      );
    } else {
      console.log(`  ${colors.green}Identical${colors.reset}`);
    }
  }
}
