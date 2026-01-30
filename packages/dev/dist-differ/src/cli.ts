/* eslint-disable no-console */
import * as fs from 'fs';
import * as path from 'path';
import {compareDirectories} from './directory';
import {compareFiles, compareFilesByPrefix} from './comparison';
import {createContext} from './context';
import {DistDifferMCPServer} from './mcp-server';
import {
  runComparison as runComparisonUtil,
  type ComparisonOptions,
} from './compare';

import {DEFAULT_OPTIONS, validateSizeThreshold} from './options';

export interface CliOptions {
  ignoreAssetIds: boolean;
  ignoreUnminifiedRefs: boolean;
  ignoreSourceMapUrl: boolean;
  ignoreSwappedVariables: boolean;
  summaryMode: boolean;
  verbose: boolean;
  jsonMode: boolean;
  mcpMode: boolean;
  sizeThreshold: number;
}

export function parseArgs(args: string[]): {
  options: CliOptions;
  files: string[];
  error?: string;
} {
  const options: CliOptions = {...DEFAULT_OPTIONS};
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
    } else if (arg === '--json') {
      options.jsonMode = true;
    } else if (arg === '--mcp') {
      options.mcpMode = true;
      options.jsonMode = true; // MCP mode requires JSON mode
    } else if (arg === '--disambiguation-size-threshold') {
      if (i + 1 >= args.length) {
        return {
          options,
          files,
          error: 'Error: --disambiguation-size-threshold requires a value',
        };
      }
      const thresholdValue = parseFloat(args[i + 1]);
      if (!validateSizeThreshold(thresholdValue)) {
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
    '  --json                                Output results in JSON format for AI analysis',
  );
  console.error(
    '  --mcp                                 Start an MCP server for AI agent queries (requires comparison paths)',
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

  const context = createContext(undefined, undefined, dir1, dir2, options);
  compareFilesByPrefix(prefix1, prefix2, dir1, dir2, context);
}

/**
 * Runs a comparison and returns the JSON report (for MCP mode)
 */
async function runComparisonForMCP(
  file1: string,
  file2: string,
  options: CliOptions,
): Promise<import('./json').JsonReport | null> {
  const comparisonOptions: ComparisonOptions = {
    ignoreAssetIds: options.ignoreAssetIds,
    ignoreUnminifiedRefs: options.ignoreUnminifiedRefs,
    ignoreSourceMapUrl: options.ignoreSourceMapUrl,
    ignoreSwappedVariables: options.ignoreSwappedVariables,
    jsonMode: options.jsonMode,
    sizeThreshold: options.sizeThreshold,
  };

  const report = await runComparisonUtil(file1, file2, comparisonOptions);

  if (!report) {
    console.error(`Error: Could not compare ${file1} and ${file2}`);
  }

  return report;
}

export async function main(): Promise<void> {
  const args = process.argv.slice(2);

  const {options, files, error} = parseArgs(args);

  if (error) {
    console.error(error);
    process.exitCode = 1;
    return;
  }

  // MCP mode: starts server (optionally with initial comparison)
  if (options.mcpMode) {
    const mcpServer = new DistDifferMCPServer();

    // If two paths are provided, run an initial comparison
    if (files.length === 2) {
      const [file1, file2] = files;

      console.error('Running initial comparison...');
      const report = await runComparisonForMCP(file1, file2, options);

      if (!report) {
        console.error(
          'Warning: Initial comparison failed, but MCP server will start anyway.',
        );
        console.error(
          'You can use the "compare" tool to run comparisons once the server is running.',
        );
      } else {
        console.error('Comparison complete.');
        console.error(
          `Summary: ${report.summary.totalHunks} total hunks (${report.summary.meaningfulHunks} meaningful, ${report.summary.harmlessHunks} harmless)`,
        );
        if (report.files) {
          console.error(`Files compared: ${report.files.length}`);
        }
        mcpServer.setReport(report);
      }
    } else if (files.length > 0) {
      console.error('Warning: --mcp expects 0 or 2 file/directory paths.');
      console.error('If you provide paths, you must provide exactly 2.');
      console.error('Starting MCP server without initial comparison.');
      console.error(
        'You can use the "compare" tool to run comparisons once the server is running.',
      );
    }

    console.error('');
    console.error('Starting MCP server...');
    console.error('The server is now ready to accept connections via stdio.');
    console.error(
      'Use the "compare" tool to run dist diff analyses between paths.',
    );
    console.error('Press Ctrl+C to stop the server.');
    console.error('');

    // Start the MCP server (this will run indefinitely)
    await mcpServer.start();
    return;
  }

  // Normal mode: run comparison and exit
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
    const context = createContext(undefined, undefined, file1, file2, options);
    compareDirectories(file1, file2, context);
    return;
  } else if (stat1.isDirectory() || stat2.isDirectory()) {
    console.error('Error: Cannot compare a directory with a file');
    console.error('  Both arguments must be either files or directories');
    process.exitCode = 1;
    return;
  }

  // Both are files - compare them
  const context = createContext(file1, file2, undefined, undefined, options);
  compareFiles(file1, file2, context);
}
