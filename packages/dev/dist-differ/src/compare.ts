/**
 * Shared comparison utilities used by both CLI and MCP server
 */
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import {getAllFiles} from './directory';
import {readAndDeminify} from './utils/deminify';
import {computeDiff} from './diff';
import {
  diffToJson,
  generateFileJsonReport,
  generateDirectoryJsonReport,
} from './json';
import {matchFilesByPrefix, extractPrefix} from './match';
import {createContext} from './context';
import type {JsonReport, JsonFileResult} from './json';

/**
 * Options for running a comparison
 */
export interface ComparisonOptions {
  ignoreAssetIds: boolean;
  ignoreUnminifiedRefs: boolean;
  ignoreSourceMapUrl: boolean;
  ignoreSwappedVariables: boolean;
  jsonMode: boolean;
  sizeThreshold: number;
}

/**
 * Expands ~ to home directory in a path
 */
export function expandTilde(filePath: string): string {
  if (filePath.startsWith('~/')) {
    return path.join(os.homedir(), filePath.slice(2));
  }
  if (filePath === '~') {
    return os.homedir();
  }
  return filePath;
}

/**
 * Runs a comparison between two paths and returns the JSON report.
 * Handles files, directories, and prefix matching.
 */
// eslint-disable-next-line require-await
export async function runComparison(
  path1: string,
  path2: string,
  options: ComparisonOptions,
): Promise<JsonReport | null> {
  // Expand ~ to home directory
  const expandedPath1 = expandTilde(path1);
  const expandedPath2 = expandTilde(path2);

  const exists1 = fs.existsSync(expandedPath1);
  const exists2 = fs.existsSync(expandedPath2);

  // If paths don't exist, try prefix matching
  if (!exists1 || !exists2) {
    return runPrefixMatching(expandedPath1, expandedPath2, options);
  }

  const stat1 = fs.statSync(expandedPath1);
  const stat2 = fs.statSync(expandedPath2);

  if (stat1.isDirectory() && stat2.isDirectory()) {
    return compareDirectories(expandedPath1, expandedPath2, options);
  } else if (stat1.isDirectory() || stat2.isDirectory()) {
    return null; // Cannot compare directory with file
  }

  // Both are files
  return compareFiles(expandedPath1, expandedPath2, options);
}

/**
 * Compares two directories and returns a JSON report
 */
function compareDirectories(
  dir1: string,
  dir2: string,
  options: ComparisonOptions,
): JsonReport | null {
  const absDir1 = path.resolve(dir1);
  const absDir2 = path.resolve(dir2);
  const files1 = getAllFiles(absDir1);
  const files2 = getAllFiles(absDir2);

  if (files1.length !== files2.length) {
    return {
      metadata: {
        dir1: absDir1,
        dir2: absDir2,
        comparisonDate: new Date().toISOString(),
        options: {
          ignoreAssetIds: options.ignoreAssetIds,
          ignoreUnminifiedRefs: options.ignoreUnminifiedRefs,
          ignoreSourceMapUrl: options.ignoreSourceMapUrl,
          ignoreSwappedVariables: options.ignoreSwappedVariables,
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
  }

  const {matched, ambiguous} = matchFilesByPrefix(
    files1,
    files2,
    options.sizeThreshold,
  );

  const context = createContext(undefined, undefined, dir1, dir2, options);
  const fileResults: JsonFileResult[] = [];

  for (const {file1: f1, file2: f2} of matched) {
    const lines1 = readAndDeminify(f1.fullPath);
    const lines2 = readAndDeminify(f2.fullPath);

    if (!lines1 || !lines2) {
      continue;
    }

    const diff = computeDiff(lines1, lines2);
    const fileResult = diffToJson(diff, f1.fullPath, f2.fullPath, context);
    fileResults.push(fileResult);
  }

  return generateDirectoryJsonReport(
    matched,
    ambiguous,
    absDir1,
    absDir2,
    context,
    fileResults,
  );
}

/**
 * Compares two files and returns a JSON report
 */
function compareFiles(
  file1: string,
  file2: string,
  options: ComparisonOptions,
): JsonReport | null {
  const absFile1 = path.resolve(file1);
  const absFile2 = path.resolve(file2);
  const lines1 = readAndDeminify(absFile1);
  const lines2 = readAndDeminify(absFile2);

  if (!lines1 || !lines2) {
    return null;
  }

  const diff = computeDiff(lines1, lines2);
  const context = createContext(file1, file2, undefined, undefined, options);
  return generateFileJsonReport(diff, absFile1, absFile2, context);
}

/**
 * Handles prefix-based file matching when paths don't exist as files
 */
function runPrefixMatching(
  path1: string,
  path2: string,
  options: ComparisonOptions,
): JsonReport | null {
  // Resolve to absolute paths first
  const absPath1 = path.resolve(path1);
  const absPath2 = path.resolve(path2);

  // Extract parent directory and prefix from each path
  const dir1 = path.dirname(absPath1);
  const dir2 = path.dirname(absPath2);
  const prefix1 = path.basename(absPath1);
  const prefix2 = path.basename(absPath2);

  // Check if parent directories exist
  if (!fs.existsSync(dir1) || !fs.statSync(dir1).isDirectory()) {
    return null;
  }

  if (!fs.existsSync(dir2) || !fs.statSync(dir2).isDirectory()) {
    return null;
  }

  // Search for files matching the prefix in both directories
  const allFiles1 = getAllFiles(dir1);
  const allFiles2 = getAllFiles(dir2);

  const files1 = allFiles1.filter((f) => {
    const filePrefix = extractPrefix(f.filename);
    return filePrefix === prefix1 && f.filename.endsWith('.js');
  });

  const files2 = allFiles2.filter((f) => {
    const filePrefix = extractPrefix(f.filename);
    return filePrefix === prefix2 && f.filename.endsWith('.js');
  });

  if (files1.length === 0 || files2.length === 0) {
    return null;
  }

  // Use disambiguation logic when multiple files match
  const {matched, ambiguous} = matchFilesByPrefix(
    files1,
    files2,
    options.sizeThreshold,
  );

  const context = createContext(undefined, undefined, dir1, dir2, options);
  const fileResults: JsonFileResult[] = [];

  for (const {file1: f1, file2: f2} of matched) {
    const lines1 = readAndDeminify(f1.fullPath);
    const lines2 = readAndDeminify(f2.fullPath);

    if (!lines1 || !lines2) {
      continue;
    }

    const diff = computeDiff(lines1, lines2);
    const fileResult = diffToJson(diff, f1.fullPath, f2.fullPath, context);
    fileResults.push(fileResult);
  }

  return generateDirectoryJsonReport(
    matched,
    ambiguous,
    dir1,
    dir2,
    context,
    fileResults,
  );
}
