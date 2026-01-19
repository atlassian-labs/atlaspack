/* eslint-disable monorepo/no-internal-import */
import {McpServer} from '@modelcontextprotocol/sdk/server/mcp.js';
import {StdioServerTransport} from '@modelcontextprotocol/sdk/server/stdio.js';
import {z} from 'zod';
import type {JsonReport, JsonFileResult, JsonHunk} from './json';
import {
  runComparison as runComparisonUtil,
  type ComparisonOptions,
} from './compare';
import {DEFAULT_OPTIONS} from './options';

/**
 * MCP server for dist-differ that allows AI agents to query dist diff data
 */
interface IteratorState {
  filePath: string;
  currentIndex: number;
  totalHunks: number;
}

export class DistDifferMCPServer {
  private server: McpServer;
  private report: JsonReport | null = null;
  private transport: StdioServerTransport;

  // State for progressive iteration
  private fileIterators: Map<string, IteratorState> = new Map();
  private globalIterator: IteratorState | null = null; // For iterating all meaningful hunks

  // Limits for large file handling
  private static readonly MAX_HUNKS_PER_FILE = 1000; // Stop processing after this many hunks
  private static readonly MAX_HUNKS_TO_DISPLAY = 10; // Show only this many hunks in detail (reduced from 20)
  private static readonly SAMPLE_SIZE = 3; // Show first N and last N hunks (reduced from 5)
  private static readonly MAX_CHANGE_LINES_PER_HUNK = 2; // Show only first 2 change lines per hunk
  private static readonly DEFAULT_BATCH_SIZE = 5; // Default number of hunks to return per iteration

  constructor() {
    this.server = new McpServer({
      name: 'atlaspack-dist-differ-mcp',
      version: '1.0.0',
    });

    this.transport = new StdioServerTransport();

    this.registerTools();
  }

  private registerTools(): void {
    // Tool to run a comparison between two paths
    this.server.registerTool(
      'compare',
      {
        title: 'Compare Two Paths',
        description:
          'Run a dist diff analysis between two files or directories. Returns summary data immediately for quick analysis. The full diff data is stored in memory for selective querying with other tools. For large comparisons (100s of MB), use specific query tools rather than loading everything at once.',
        inputSchema: {
          path1: z.string().describe('First file or directory path to compare'),
          path2: z
            .string()
            .describe('Second file or directory path to compare'),
          ignoreAssetIds: z
            .boolean()
            .optional()
            .describe('Ignore asset ID differences'),
          ignoreUnminifiedRefs: z
            .boolean()
            .optional()
            .describe('Ignore unminified ref differences'),
          ignoreSourceMapUrl: z
            .boolean()
            .optional()
            .describe('Ignore source map URL differences'),
          ignoreSwappedVariables: z
            .boolean()
            .optional()
            .describe('Ignore swapped variable differences'),
        },
      },
      async ({
        path1,
        path2,
        ignoreAssetIds,
        ignoreUnminifiedRefs,
        ignoreSourceMapUrl,
        ignoreSwappedVariables,
      }) => {
        try {
          const options: ComparisonOptions = {
            ignoreAssetIds: ignoreAssetIds ?? false,
            ignoreUnminifiedRefs: ignoreUnminifiedRefs ?? false,
            ignoreSourceMapUrl: ignoreSourceMapUrl ?? false,
            ignoreSwappedVariables: ignoreSwappedVariables ?? false,
            jsonMode: true,
            sizeThreshold: DEFAULT_OPTIONS.sizeThreshold,
          };

          const report = await runComparisonUtil(path1, path2, options);

          if (!report) {
            return {
              content: [
                {
                  type: 'text',
                  text: `Error: Could not compare ${path1} and ${path2}. Please check that both paths exist and are valid files or directories.`,
                },
              ],
            };
          }

          // Store the report for other tools to use
          this.report = report;

          // Reset all iterators when a new comparison is done
          this.fileIterators.clear();
          this.globalIterator = null;

          const summary = report.summary;
          const fileCount = report.files?.length ?? 0;
          const hasLargeDataset = fileCount > 100 || summary.totalHunks > 1000;

          // Check for global object transformations across all files
          // IMPORTANT: Check both hunks AND stored transformations (for truncated files)
          const globalObjectTransformations: Array<{
            file: string;
            hunk?: JsonHunk;
            line?: string;
            lineNum?: number;
          }> = [];
          if (report.files) {
            for (const file of report.files) {
              // Check hunks in the report
              for (const hunk of file.hunks) {
                if (
                  hunk.category === 'meaningful' &&
                  hunk.analysis?.changeType === 'global_object_transformation'
                ) {
                  globalObjectTransformations.push({file: file.path, hunk});
                }
              }

              // Check stored transformations (for truncated files where hunk wasn't processed)
              const storedTransformations = (file as any)
                .globalObjectTransformations as
                | Array<{line: string; lineNum: number; type: 'remove' | 'add'}>
                | undefined;
              if (storedTransformations && storedTransformations.length > 0) {
                for (const transformation of storedTransformations) {
                  globalObjectTransformations.push({
                    file: file.path,
                    line: transformation.line,
                    lineNum: transformation.lineNum,
                  });
                }
              }
            }
          }

          return {
            content: [
              {
                type: 'text',
                text: [
                  `Comparison complete between:`,
                  `  ${path1}`,
                  `  ${path2}`,
                  ``,
                  ...(globalObjectTransformations.length > 0
                    ? [
                        `🚨 CRITICAL BREAKING CHANGES DETECTED:`,
                        `   ${globalObjectTransformations.length} global object transformation(s) found!`,
                        `   These will cause RUNTIME ERRORS. Global objects (globalThis, window, document, etc.)`,
                        `   have been incorrectly transformed into scoped variables.`,
                        `   Example: globalThis.__SSR_ATL_TOKEN__ → $hash$var$globalThis.__SSR_ATL_TOKEN__`,
                        ``,
                        `   Affected files:`,
                        ...globalObjectTransformations
                          .slice(0, 10)
                          .map(({file}) => `     - ${file}`),
                        ...(globalObjectTransformations.length > 10
                          ? [
                              `     ... and ${globalObjectTransformations.length - 10} more`,
                            ]
                          : []),
                        ``,
                      ]
                    : []),
                  `Summary (returned immediately for analysis):`,
                  `- Total hunks: ${summary.totalHunks}`,
                  `- Meaningful hunks: ${summary.meaningfulHunks}`,
                  `- Harmless hunks: ${summary.harmlessHunks}`,
                  `- Identical: ${summary.identical}`,
                  ...(globalObjectTransformations.length > 0
                    ? [
                        `- ⚠️  Global object transformations: ${globalObjectTransformations.length} (CRITICAL)`,
                      ]
                    : []),
                  summary.identicalFiles !== undefined
                    ? `- Identical files: ${summary.identicalFiles}`
                    : '',
                  summary.differentFiles !== undefined
                    ? `- Different files: ${summary.differentFiles}`
                    : '',
                  summary.totalFiles !== undefined
                    ? `- Total files: ${summary.totalFiles}`
                    : '',
                  summary.error ? `- Error: ${summary.error}` : '',
                  ``,
                  hasLargeDataset
                    ? `Note: This is a large comparison (${fileCount} files, ${summary.totalHunks} hunks). The full diff data is stored in memory for selective querying. Use specific tools like 'get-meaningful-changes' or 'get-file-details' to explore specific areas rather than loading everything at once.`
                    : `The full comparison results are now available in memory. Use other tools like 'get-summary', 'list-files', 'get-meaningful-changes', or 'get-file-details' to explore the differences selectively.`,
                ]
                  .filter(Boolean)
                  .join('\n'),
              },
            ],
          };
        } catch (error) {
          return {
            content: [
              {
                type: 'text',
                text: `Error running comparison: ${error instanceof Error ? error.message : String(error)}`,
              },
            ],
          };
        }
      },
    );

    // Tool to get the summary of the comparison (statistics only, no hunk details)
    this.server.registerTool(
      'get-summary',
      {
        title: 'Get Comparison Summary',
        description:
          'Get a high-level summary of the dist comparison (statistics only - no detailed hunk information). Use this for large comparisons to avoid overwhelming the response. For details, use get-meaningful-changes or get-file-details with specific files.',
        inputSchema: {},
      },
      () => {
        if (!this.report) {
          return {
            content: [
              {
                type: 'text',
                text: 'No comparison data available. Run a comparison first.',
              },
            ],
          };
        }

        const summary = this.report.summary;

        // Check for global object transformations across all files
        // IMPORTANT: Check both hunks AND stored transformations (for truncated files)
        const globalObjectTransformations: Array<{
          file: string;
          hunk?: JsonHunk;
          line?: string;
          lineNum?: number;
        }> = [];
        if (this.report.files) {
          for (const file of this.report.files) {
            // Check hunks in the report
            for (const hunk of file.hunks) {
              if (
                hunk.category === 'meaningful' &&
                hunk.analysis?.changeType === 'global_object_transformation'
              ) {
                globalObjectTransformations.push({file: file.path, hunk});
              }
            }

            // Check stored transformations (for truncated files where hunk wasn't processed)
            const storedTransformations = (file as any)
              .globalObjectTransformations as
              | Array<{line: string; lineNum: number; type: 'remove' | 'add'}>
              | undefined;
            if (storedTransformations && storedTransformations.length > 0) {
              for (const transformation of storedTransformations) {
                globalObjectTransformations.push({
                  file: file.path,
                  line: transformation.line,
                  lineNum: transformation.lineNum,
                });
              }
            }
          }
        }

        return {
          content: [
            {
              type: 'text',
              text: [
                ...(globalObjectTransformations.length > 0
                  ? [
                      `🚨 CRITICAL BREAKING CHANGES DETECTED:`,
                      `   ${globalObjectTransformations.length} global object transformation(s) found!`,
                      `   These will cause RUNTIME ERRORS. Global objects (globalThis, window, document, etc.)`,
                      `   have been incorrectly transformed into scoped variables.`,
                      `   Example: globalThis.__SSR_ATL_TOKEN__ → $hash$var$globalThis.__SSR_ATL_TOKEN__`,
                      ``,
                    ]
                  : []),
                `Comparison Summary:`,
                `- Total hunks: ${summary.totalHunks}`,
                `- Meaningful hunks: ${summary.meaningfulHunks}`,
                `- Harmless hunks: ${summary.harmlessHunks}`,
                `- Identical: ${summary.identical}`,
                ...(globalObjectTransformations.length > 0
                  ? [
                      `- ⚠️  Global object transformations: ${globalObjectTransformations.length} (CRITICAL)`,
                    ]
                  : []),
                summary.identicalFiles !== undefined
                  ? `- Identical files: ${summary.identicalFiles}`
                  : '',
                summary.differentFiles !== undefined
                  ? `- Different files: ${summary.differentFiles}`
                  : '',
                summary.totalFiles !== undefined
                  ? `- Total files: ${summary.totalFiles}`
                  : '',
                summary.error ? `- Error: ${summary.error}` : '',
              ]
                .filter(Boolean)
                .join('\n'),
            },
          ],
        };
      },
    );

    // Tool to list all files in the comparison
    this.server.registerTool(
      'list-files',
      {
        title: 'List Compared Files',
        description:
          'List all files that were compared, showing their status (identical or different) and hunk counts.',
        inputSchema: {},
      },
      () => {
        if (!this.report || !this.report.files) {
          return {
            content: [
              {
                type: 'text',
                text: 'No comparison data available. Run a comparison first.',
              },
            ],
          };
        }

        const files = this.report.files;
        return {
          content: [
            {
              type: 'text',
              text: [
                `Compared Files (${files.length} total):`,
                ...files.map(
                  (f) =>
                    `- ${f.path} [${f.status}] - ${f.hunkCount} hunks (${f.meaningfulHunkCount} meaningful, ${f.harmlessHunkCount} harmless)`,
                ),
              ].join('\n'),
            },
          ],
        };
      },
    );

    // Tool to get details about a specific file
    this.server.registerTool(
      'get-file-details',
      {
        title: 'Get File Comparison Details',
        description:
          'Get detailed information about the differences in a specific file, including all hunks.',
        inputSchema: {
          filePath: z
            .string()
            .describe('The relative path of the file to query'),
        },
      },
      ({filePath}) => {
        if (!this.report || !this.report.files) {
          return {
            content: [
              {
                type: 'text',
                text: 'No comparison data available. Run a comparison first.',
              },
            ],
          };
        }

        const file = this.report.files.find((f) => f.path === filePath);
        if (!file) {
          return {
            content: [
              {
                type: 'text',
                text: `File not found: ${filePath}`,
              },
            ],
          };
        }

        const isLargeFile =
          file.hunkCount > DistDifferMCPServer.MAX_HUNKS_TO_DISPLAY;
        const isTruncated = (file as any).truncated === true;
        const hunksToShow = this.sampleHunks(file.hunks, isLargeFile);

        return {
          content: [
            {
              type: 'text',
              text: [
                `File: ${file.path}`,
                `Status: ${file.status}`,
                `Total hunks: ${file.hunkCount}${isTruncated ? ' (truncated - file has 1000+ hunks)' : ''}`,
                `Meaningful hunks: ${file.meaningfulHunkCount}`,
                `Harmless hunks: ${file.harmlessHunkCount}`,
                '',
                isLargeFile || isTruncated
                  ? `Note: This file has ${isTruncated ? '1000+' : file.hunkCount} hunks. Showing minimal sample (first ${DistDifferMCPServer.SAMPLE_SIZE} and last ${DistDifferMCPServer.SAMPLE_SIZE}) to avoid overwhelming the response. For specific patterns, use 'search-changes' with a pattern.`
                  : 'Hunks:',
                '',
                ...hunksToShow.map((hunk) =>
                  this.formatHunk(hunk, !isLargeFile && !isTruncated),
                ),
                isLargeFile || isTruncated
                  ? `\n... (${isTruncated ? '1000+' : file.hunkCount - DistDifferMCPServer.MAX_HUNKS_TO_DISPLAY} more hunks not shown)`
                  : '',
              ]
                .filter(Boolean)
                .join('\n'),
            },
          ],
        };
      },
    );

    // Tool to get meaningful changes only (statistics only - use get-next-meaningful-hunks for iteration)
    this.server.registerTool(
      'get-meaningful-changes',
      {
        title: 'Get Meaningful Changes Summary',
        description:
          'Get a summary of meaningful (non-harmless) changes across all files. Returns statistics only to avoid overwhelming the response. Use "get-next-meaningful-hunks" for progressive iteration through the actual changes.',
        inputSchema: {},
      },
      () => {
        if (!this.report || !this.report.files) {
          return {
            content: [
              {
                type: 'text',
                text: 'No comparison data available. Run a comparison first.',
              },
            ],
          };
        }

        const meaningfulHunks: Array<{file: string; hunk: JsonHunk}> = [];
        for (const file of this.report.files) {
          for (const hunk of file.hunks) {
            if (hunk.category === 'meaningful') {
              meaningfulHunks.push({file: file.path, hunk});
            }
          }
        }

        if (meaningfulHunks.length === 0) {
          return {
            content: [
              {
                type: 'text',
                text: 'No meaningful changes found. All differences are harmless.',
              },
            ],
          };
        }

        // Always return statistics only - never return hunk details to avoid context window issues
        const filesWithChanges = new Set(meaningfulHunks.map((m) => m.file));
        const fileCounts = meaningfulHunks.reduce((acc, {file}) => {
          acc.set(file, (acc.get(file) || 0) + 1);
          return acc;
        }, new Map<string, number>());

        // Group by change type for better insights
        const changeTypes = meaningfulHunks.reduce((acc, {hunk}) => {
          const type = hunk.analysis?.changeType || 'unknown';
          acc.set(type, (acc.get(type) || 0) + 1);
          return acc;
        }, new Map<string, number>());

        // Count critical global object transformations
        // Check both hunks AND stored transformations (for truncated files)
        const globalObjectTransformationsFromHunks = meaningfulHunks.filter(
          ({hunk}) =>
            hunk.analysis?.changeType === 'global_object_transformation',
        );

        // Also check stored transformations in file results (for truncated files)
        const globalObjectTransformationsFromFiles: Array<{file: string}> = [];
        if (this.report.files) {
          for (const file of this.report.files) {
            const storedTransformations = (file as any)
              .globalObjectTransformations as
              | Array<{line: string; lineNum: number; type: 'remove' | 'add'}>
              | undefined;
            if (storedTransformations && storedTransformations.length > 0) {
              // Only count if not already in meaningfulHunks
              const alreadyCounted = meaningfulHunks.some(
                ({file: f, hunk}) =>
                  f === file.path &&
                  hunk.analysis?.changeType === 'global_object_transformation',
              );
              if (!alreadyCounted) {
                globalObjectTransformationsFromFiles.push({file: file.path});
              }
            }
          }
        }

        const globalObjectTransformations = [
          ...globalObjectTransformationsFromHunks,
          ...globalObjectTransformationsFromFiles,
        ];

        const impactCounts = meaningfulHunks.reduce((acc, {hunk}) => {
          const impact = hunk.analysis?.impact || 'unknown';
          acc.set(impact, (acc.get(impact) || 0) + 1);
          return acc;
        }, new Map<string, number>());

        return {
          content: [
            {
              type: 'text',
              text: [
                `Meaningful Changes Summary:`,
                `- Total meaningful hunks: ${meaningfulHunks.length}`,
                `- Files with meaningful changes: ${filesWithChanges.size}`,
                ``,
                `Change types:`,
                ...Array.from(changeTypes.entries())
                  .sort((a, b) => b[1] - a[1])
                  .map(([type, count]) => `  - ${type}: ${count}`),
                ``,
                ...(globalObjectTransformations.length > 0
                  ? [
                      `⚠️  CRITICAL: ${globalObjectTransformations.length} global object transformation(s) detected!`,
                      `   These are BREAKING CHANGES that will cause runtime errors.`,
                      `   Global objects (globalThis, window, document, etc.) have been incorrectly`,
                      `   transformed into scoped variables, breaking runtime access.`,
                      ``,
                    ]
                  : []),
                `Impact distribution:`,
                ...Array.from(impactCounts.entries())
                  .sort((a, b) => {
                    const order = {high: 0, medium: 1, low: 2, unknown: 3};
                    return (
                      (order[a[0] as keyof typeof order] ?? 99) -
                      (order[b[0] as keyof typeof order] ?? 99)
                    );
                  })
                  .map(([impact, count]) => `  - ${impact}: ${count}`),
                ``,
                `Top files by meaningful change count:`,
                ...Array.from(fileCounts.entries())
                  .sort((a, b) => b[1] - a[1])
                  .slice(0, 10)
                  .map(
                    ([file, count]) => `  - ${file}: ${count} meaningful hunks`,
                  ),
                ``,
                `To review the actual changes:`,
                `- Use 'get-next-meaningful-hunks' to iterate through changes progressively (5 at a time)`,
                `- Use 'get-file-details <filePath>' to see all changes in a specific file`,
                `- Use 'get-hunk-by-id <hunkId>' to examine a specific hunk`,
                `- Use 'search-changes <pattern>' to find specific patterns`,
              ].join('\n'),
            },
          ],
        };
      },
    );

    // Tool to search for specific patterns in changes
    this.server.registerTool(
      'search-changes',
      {
        title: 'Search Changes',
        description:
          'Search for specific text patterns in the changes across all files.',
        inputSchema: {
          pattern: z.string().describe('The text pattern to search for'),
          filePath: z
            .string()
            .optional()
            .describe('Optional: limit search to a specific file'),
        },
      },
      ({pattern, filePath}) => {
        if (!this.report || !this.report.files) {
          return {
            content: [
              {
                type: 'text',
                text: 'No comparison data available. Run a comparison first.',
              },
            ],
          };
        }

        const searchPattern = new RegExp(pattern, 'i');
        const matches: Array<{file: string; hunk: JsonHunk}> = [];

        const filesToSearch = filePath
          ? this.report.files.filter((f) => f.path === filePath)
          : this.report.files;

        for (const file of filesToSearch) {
          for (const hunk of file.hunks) {
            const hunkText = hunk.changes.map((c) => c.line).join(' ');
            if (searchPattern.test(hunkText)) {
              matches.push({file: file.path, hunk});
            }
          }
        }

        if (matches.length === 0) {
          return {
            content: [
              {
                type: 'text',
                text: `No matches found for pattern: ${pattern}`,
              },
            ],
          };
        }

        return {
          content: [
            {
              type: 'text',
              text: [
                `Matches for "${pattern}" (${matches.length} found):`,
                '',
                ...matches.map(
                  ({file, hunk}) => `File: ${file}\n${this.formatHunk(hunk)}\n`,
                ),
              ].join('\n'),
            },
          ],
        };
      },
    );

    // Tool to get next batch of hunks for a specific file (progressive iteration)
    this.server.registerTool(
      'get-next-hunks',
      {
        title: 'Get Next Batch of Hunks',
        description:
          'Get the next batch of hunks for a specific file, allowing progressive iteration through large diffs. Use this to navigate through changes incrementally without loading everything at once.',
        inputSchema: {
          filePath: z.string().describe('The file path to iterate through'),
          batchSize: z
            .number()
            .optional()
            .describe('Number of hunks to return (default: 5)'),
          reset: z
            .boolean()
            .optional()
            .describe('Reset iterator to beginning (default: false)'),
        },
      },
      ({
        filePath,
        batchSize = DistDifferMCPServer.DEFAULT_BATCH_SIZE,
        reset = false,
      }) => {
        if (!this.report || !this.report.files) {
          return {
            content: [
              {
                type: 'text',
                text: 'No comparison data available. Run a comparison first using the "compare" tool.',
              },
            ],
          };
        }

        const file = this.report.files.find((f) => f.path === filePath);
        if (!file) {
          return {
            content: [
              {
                type: 'text',
                text: `File not found: ${filePath}`,
              },
            ],
          };
        }

        // Get or create iterator state
        let iterator = this.fileIterators.get(filePath);
        if (!iterator || reset) {
          iterator = {
            filePath,
            currentIndex: 0,
            totalHunks: file.hunks.length,
          };
          this.fileIterators.set(filePath, iterator);
        }

        if (iterator.currentIndex >= iterator.totalHunks) {
          return {
            content: [
              {
                type: 'text',
                text: `Reached end of file. All ${iterator.totalHunks} hunks have been shown. Use reset=true to start over.`,
              },
            ],
          };
        }

        const endIndex = Math.min(
          iterator.currentIndex + batchSize,
          iterator.totalHunks,
        );
        const hunksToShow = file.hunks.slice(iterator.currentIndex, endIndex);

        // Update iterator
        iterator.currentIndex = endIndex;

        return {
          content: [
            {
              type: 'text',
              text: [
                `File: ${file.path}`,
                `Showing hunks ${iterator.currentIndex - hunksToShow.length + 1}-${iterator.currentIndex} of ${iterator.totalHunks}`,
                `Progress: ${iterator.currentIndex}/${iterator.totalHunks} (${Math.round((iterator.currentIndex / iterator.totalHunks) * 100)}%)`,
                '',
                'Hunks:',
                ...hunksToShow.map((hunk) => this.formatHunk(hunk, true)),
                '',
                iterator.currentIndex < iterator.totalHunks
                  ? `Use 'get-next-hunks' again to see the next ${Math.min(batchSize, iterator.totalHunks - iterator.currentIndex)} hunks.`
                  : 'All hunks shown. Use reset=true to start over.',
              ].join('\n'),
            },
          ],
        };
      },
    );

    // Tool to get next batch of meaningful hunks across all files
    this.server.registerTool(
      'get-next-meaningful-hunks',
      {
        title: 'Get Next Batch of Meaningful Hunks',
        description:
          'Get the next batch of meaningful hunks across all files, allowing progressive iteration. This is useful for large comparisons where you want to review meaningful changes incrementally.',
        inputSchema: {
          batchSize: z
            .number()
            .optional()
            .describe('Number of hunks to return (default: 5)'),
          reset: z
            .boolean()
            .optional()
            .describe('Reset iterator to beginning (default: false)'),
        },
      },
      ({batchSize = DistDifferMCPServer.DEFAULT_BATCH_SIZE, reset = false}) => {
        if (!this.report || !this.report.files) {
          return {
            content: [
              {
                type: 'text',
                text: 'No comparison data available. Run a comparison first using the "compare" tool.',
              },
            ],
          };
        }

        // Collect all meaningful hunks
        const meaningfulHunks: Array<{
          file: string;
          hunk: JsonHunk;
          index: number;
        }> = [];
        let globalIndex = 0;
        for (const file of this.report.files) {
          for (const hunk of file.hunks) {
            if (hunk.category === 'meaningful') {
              meaningfulHunks.push({
                file: file.path,
                hunk,
                index: globalIndex++,
              });
            }
          }
        }

        if (meaningfulHunks.length === 0) {
          return {
            content: [
              {
                type: 'text',
                text: 'No meaningful changes found. All differences are harmless.',
              },
            ],
          };
        }

        // Get or create global iterator
        if (!this.globalIterator || reset) {
          this.globalIterator = {
            filePath: 'all',
            currentIndex: 0,
            totalHunks: meaningfulHunks.length,
          };
        }

        if (
          this.globalIterator.currentIndex >= this.globalIterator.totalHunks
        ) {
          return {
            content: [
              {
                type: 'text',
                text: `Reached end. All ${this.globalIterator.totalHunks} meaningful hunks have been shown. Use reset=true to start over.`,
              },
            ],
          };
        }

        const endIndex = Math.min(
          this.globalIterator.currentIndex + batchSize,
          this.globalIterator.totalHunks,
        );
        const hunksToShow = meaningfulHunks.slice(
          this.globalIterator.currentIndex,
          endIndex,
        );

        // Update iterator
        this.globalIterator.currentIndex = endIndex;

        return {
          content: [
            {
              type: 'text',
              text: [
                `Meaningful Changes`,
                `Showing ${this.globalIterator.currentIndex - hunksToShow.length + 1}-${this.globalIterator.currentIndex} of ${this.globalIterator.totalHunks} meaningful hunks`,
                `Progress: ${this.globalIterator.currentIndex}/${this.globalIterator.totalHunks} (${Math.round((this.globalIterator.currentIndex / this.globalIterator.totalHunks) * 100)}%)`,
                '',
                ...hunksToShow.map(
                  ({file, hunk}) =>
                    `File: ${file}\n${this.formatHunk(hunk, true)}\n`,
                ),
                '',
                this.globalIterator.currentIndex <
                this.globalIterator.totalHunks
                  ? `Use 'get-next-meaningful-hunks' again to see the next ${Math.min(batchSize, this.globalIterator.totalHunks - this.globalIterator.currentIndex)} hunks.`
                  : 'All meaningful hunks shown. Use reset=true to start over.',
              ].join('\n'),
            },
          ],
        };
      },
    );

    // Tool to get a specific hunk by ID
    this.server.registerTool(
      'get-hunk-by-id',
      {
        title: 'Get Hunk by ID',
        description:
          'Get detailed information about a specific hunk by its ID. Useful when you want to examine a particular change in detail.',
        inputSchema: {
          hunkId: z.string().describe('The hunk ID (e.g., "hunk-1")'),
          filePath: z
            .string()
            .optional()
            .describe(
              'Optional: the file path to search in (searches all files if not provided)',
            ),
        },
      },
      ({hunkId, filePath}) => {
        if (!this.report || !this.report.files) {
          return {
            content: [
              {
                type: 'text',
                text: 'No comparison data available. Run a comparison first using the "compare" tool.',
              },
            ],
          };
        }

        const filesToSearch = filePath
          ? this.report.files.filter((f) => f.path === filePath)
          : this.report.files;

        for (const file of filesToSearch) {
          const hunk = file.hunks.find((h) => h.id === hunkId);
          if (hunk) {
            return {
              content: [
                {
                  type: 'text',
                  text: [
                    `Hunk ${hunkId} in file: ${file.path}`,
                    '',
                    this.formatHunk(hunk, true),
                  ].join('\n'),
                },
              ],
            };
          }
        }

        return {
          content: [
            {
              type: 'text',
              text: `Hunk ${hunkId} not found${filePath ? ` in file ${filePath}` : ''}.`,
            },
          ],
        };
      },
    );

    // Tool to quit the MCP server
    this.server.registerTool(
      'quit',
      {
        title: 'Quit MCP Server',
        description:
          'Forces the MCP server to quit and exit. Use this when you want to stop the server.',
        inputSchema: {},
      },
      () => {
        // Exit the process immediately
        // Note: We use setImmediate to allow the response to be sent before exit
        setImmediate(() => {
          process.exit(0);
        });

        return {
          content: [
            {
              type: 'text',
              text: 'MCP server shutting down...',
            },
          ],
        };
      },
    );
  }

  /**
   * Samples hunks for large files - shows first N and last N hunks
   */
  private sampleHunks(hunks: JsonHunk[], isLarge: boolean): JsonHunk[] {
    if (!isLarge || hunks.length <= DistDifferMCPServer.MAX_HUNKS_TO_DISPLAY) {
      return hunks;
    }

    const first = hunks.slice(0, DistDifferMCPServer.SAMPLE_SIZE);
    const last = hunks.slice(-DistDifferMCPServer.SAMPLE_SIZE);
    return [...first, ...last];
  }

  private formatHunk(hunk: JsonHunk, includeChanges: boolean = true): string {
    const lines = [
      `  Hunk ${hunk.id}:`,
      `    Category: ${hunk.category}${hunk.harmlessType ? ` (${hunk.harmlessType})` : ''}`,
      `    Lines: ${hunk.startLine1}-${hunk.endLine1} → ${hunk.startLine2}-${hunk.endLine2}`,
    ];

    if (hunk.analysis) {
      lines.push(
        `    Type: ${hunk.analysis.changeType || 'unknown'}, Impact: ${hunk.analysis.impact || 'unknown'}`,
      );
      // Add description if available (especially important for global object transformations)
      if (hunk.analysis.description) {
        lines.push(`    ⚠️  ${hunk.analysis.description}`);
      }
    }

    if (includeChanges && hunk.changes.length > 0) {
      // Show only first 2 change lines, very truncated
      const changesToShow = hunk.changes.slice(
        0,
        DistDifferMCPServer.MAX_CHANGE_LINES_PER_HUNK,
      );
      lines.push('    Sample changes:');
      for (const change of changesToShow) {
        const truncated = change.line.substring(0, 60);
        lines.push(
          `      ${change.type}: ${truncated}${change.line.length > 60 ? '...' : ''}`,
        );
      }
      if (hunk.changes.length > DistDifferMCPServer.MAX_CHANGE_LINES_PER_HUNK) {
        lines.push(
          `      ... (${hunk.changes.length - DistDifferMCPServer.MAX_CHANGE_LINES_PER_HUNK} more)`,
        );
      }
    }

    return lines.join('\n');
  }

  /**
   * Sets the comparison report data
   */
  setReport(report: JsonReport): void {
    this.report = report;
  }

  /**
   * Starts the MCP server
   * Note: connect() automatically calls start() on the transport
   */
  async start(): Promise<void> {
    await this.server.connect(this.transport);
    // Note: We don't log here because stdout is used for MCP protocol
    // Startup messages are logged in cli.ts before calling start()
  }
}
