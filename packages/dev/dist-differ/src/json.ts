import type {DiffEntry, FileInfo, MatchedPair, AmbiguousMatch} from './types';
import {
  isHunkOnlyAssetIds,
  isHunkOnlyUnminifiedRefs,
  isHunkOnlySourceMapUrl,
  isHunkOnlySwappedVariables,
  filterHunkEntries,
} from './hunk';
import {
  normalizeAssetIds,
  normalizeUnminifiedRefs,
  normalizeSourceMapUrl,
} from './normalize';
import type {ComparisonContext} from './context';
import * as path from 'path';

export interface JsonHunk {
  id: string;
  startLine1: number;
  endLine1: number;
  startLine2: number;
  endLine2: number;
  category: 'meaningful' | 'harmless';
  harmlessType?:
    | 'asset_ids'
    | 'unminified_refs'
    | 'source_map_url'
    | 'swapped_variables';
  confidence: number;
  context: {
    before: string[];
    after: string[];
  };
  changes: Array<{
    type: 'remove' | 'add';
    line: string;
    lineNum: number;
  }>;
  normalized?: {
    before: string;
    after: string;
  };
  analysis?: {
    semanticChange: boolean;
    changeType?: string;
    impact?: 'low' | 'medium' | 'high';
  };
}

export interface JsonFileResult {
  path: string;
  status: 'identical' | 'different';
  hunks: JsonHunk[];
  hunkCount: number;
  meaningfulHunkCount: number;
  harmlessHunkCount: number;
}

export interface JsonReport {
  metadata: {
    file1?: string;
    file2?: string;
    dir1?: string;
    dir2?: string;
    comparisonDate: string;
    options: {
      ignoreAssetIds: boolean;
      ignoreUnminifiedRefs: boolean;
      ignoreSourceMapUrl: boolean;
      ignoreSwappedVariables: boolean;
    };
  };
  summary: {
    totalHunks: number;
    meaningfulHunks: number;
    harmlessHunks: number;
    identical: boolean;
    identicalFiles?: number;
    differentFiles?: number;
    totalFiles?: number;
    error?: string;
    files1Count?: number;
    files2Count?: number;
  };
  files?: JsonFileResult[];
  ambiguousMatches?: Array<{
    prefix: string;
    dirPath: string;
    files1: Array<{path: string; size: number}>;
    files2: Array<{path: string; size: number}>;
  }>;
}

/**
 * Categorizes a hunk as meaningful or harmless
 */
function categorizeHunk(
  hunk: DiffEntry[],
  ignoreAssetIds: boolean,
  ignoreUnminifiedRefs: boolean,
  ignoreSourceMapUrl: boolean,
  ignoreSwappedVariables: boolean,
): {
  category: 'meaningful' | 'harmless';
  harmlessType?:
    | 'asset_ids'
    | 'unminified_refs'
    | 'source_map_url'
    | 'swapped_variables';
  confidence: number;
} {
  if (hunk.length === 0) {
    return {category: 'meaningful', confidence: 1.0};
  }

  // Check each harmless type
  if (ignoreAssetIds && isHunkOnlyAssetIds(hunk)) {
    return {category: 'harmless', harmlessType: 'asset_ids', confidence: 0.95};
  }
  if (ignoreUnminifiedRefs && isHunkOnlyUnminifiedRefs(hunk)) {
    return {
      category: 'harmless',
      harmlessType: 'unminified_refs',
      confidence: 0.95,
    };
  }
  if (ignoreSourceMapUrl && isHunkOnlySourceMapUrl(hunk)) {
    return {
      category: 'harmless',
      harmlessType: 'source_map_url',
      confidence: 0.98,
    };
  }
  if (ignoreSwappedVariables && isHunkOnlySwappedVariables(hunk)) {
    return {
      category: 'harmless',
      harmlessType: 'swapped_variables',
      confidence: 0.9,
    };
  }

  // Check if filtered hunk is empty (all pairs were filtered)
  const {filtered} = filterHunkEntries(
    hunk,
    ignoreAssetIds,
    ignoreUnminifiedRefs,
    ignoreSourceMapUrl,
    ignoreSwappedVariables,
  );

  if (filtered.length === 0) {
    // All pairs were filtered, but we couldn't categorize by type
    // This might be a combination of harmless changes
    return {category: 'harmless', confidence: 0.85};
  }

  return {category: 'meaningful', confidence: 1.0};
}

/**
 * Gets normalized versions of lines for a hunk
 */
function getNormalizedHunk(
  hunk: DiffEntry[],
  harmlessType?:
    | 'asset_ids'
    | 'unminified_refs'
    | 'source_map_url'
    | 'swapped_variables',
): {before: string; after: string} | undefined {
  if (!harmlessType || hunk.length === 0) {
    return undefined;
  }

  const removes = hunk.filter((e) => e.type === 'remove');
  const adds = hunk.filter((e) => e.type === 'add');

  if (removes.length === 0 || adds.length === 0) {
    return undefined;
  }

  // For simplicity, normalize the first remove/add pair
  const removeLine = removes[0].line;
  const addLine = adds[0].line;

  let normalizedBefore: string;
  let normalizedAfter: string;

  switch (harmlessType) {
    case 'asset_ids':
      normalizedBefore = normalizeAssetIds(removeLine);
      normalizedAfter = normalizeAssetIds(addLine);
      break;
    case 'unminified_refs':
      normalizedBefore = normalizeUnminifiedRefs(removeLine);
      normalizedAfter = normalizeUnminifiedRefs(addLine);
      break;
    case 'source_map_url':
      normalizedBefore = normalizeSourceMapUrl(removeLine);
      normalizedAfter = normalizeSourceMapUrl(addLine);
      break;
    case 'swapped_variables':
      // For swapped variables, normalization is more complex
      // We'll just show that they're the same after normalization
      normalizedBefore = removeLine;
      normalizedAfter = removeLine; // They should be functionally identical
      break;
    default:
      return undefined;
  }

  return {before: normalizedBefore, after: normalizedAfter};
}

/**
 * Analyzes a meaningful hunk to determine change type and impact
 */
function analyzeMeaningfulHunk(hunk: DiffEntry[]): {
  semanticChange: boolean;
  changeType?: string;
  impact?: 'low' | 'medium' | 'high';
} {
  const removes = hunk.filter((e) => e.type === 'remove');
  const adds = hunk.filter((e) => e.type === 'add');

  // Simple heuristics for analysis
  const removeText = removes.map((e) => e.line).join(' ');
  const addText = adds.map((e) => e.line).join(' ');

  // Check for function changes
  if (removeText.includes('function') || addText.includes('function')) {
    return {
      semanticChange: true,
      changeType: 'function_definition',
      impact: 'high',
    };
  }

  // Check for import/require changes
  if (removeText.includes('require') || removeText.includes('import')) {
    return {
      semanticChange: true,
      changeType: 'dependency_change',
      impact: 'medium',
    };
  }

  // Check for return statement changes
  if (removeText.includes('return') || addText.includes('return')) {
    return {
      semanticChange: true,
      changeType: 'return_value',
      impact: 'high',
    };
  }

  // Default to semantic change with medium impact
  return {
    semanticChange: true,
    changeType: 'code_change',
    impact: 'medium',
  };
}

/**
 * Extracts context lines from diff
 */
function extractContext(
  diff: DiffEntry[],
  hunkStartIndex: number,
  hunkEndIndex: number,
  contextLines: number = 3,
): {before: string[]; after: string[]} {
  const before: string[] = [];
  const after: string[] = [];

  // Get context before
  for (
    let i = Math.max(0, hunkStartIndex - contextLines);
    i < hunkStartIndex;
    i++
  ) {
    if (diff[i] && diff[i].type === 'equal') {
      before.push(diff[i].line);
    }
  }

  // Get context after
  for (
    let i = hunkEndIndex + 1;
    i < Math.min(diff.length, hunkEndIndex + 1 + contextLines);
    i++
  ) {
    if (diff[i] && diff[i].type === 'equal') {
      after.push(diff[i].line);
    }
  }

  return {before, after};
}

/**
 * Converts a diff to JSON format
 */
export function diffToJson(
  diff: DiffEntry[],
  file1: string,
  file2: string,
  context: ComparisonContext,
): JsonFileResult {
  const hunks: JsonHunk[] = [];
  let hunkId = 0;
  let currentHunk: DiffEntry[] = [];
  let hunkStartIndex = -1;
  let hunkStartLine1: number | null = null;
  let hunkStartLine2: number | null = null;

  // Group diff entries into hunks
  for (let i = 0; i < diff.length; i++) {
    const entry = diff[i];

    if (entry.type === 'equal') {
      if (currentHunk.length > 0) {
        // End of hunk - process it
        const hunkEndIndex = i - 1;
        const categorization = categorizeHunk(
          currentHunk,
          context.ignoreAssetIds,
          context.ignoreUnminifiedRefs,
          context.ignoreSourceMapUrl,
          context.ignoreSwappedVariables,
        );

        // Only include hunks that aren't fully filtered
        const {filtered} = filterHunkEntries(
          currentHunk,
          context.ignoreAssetIds,
          context.ignoreUnminifiedRefs,
          context.ignoreSourceMapUrl,
          context.ignoreSwappedVariables,
        );

        if (filtered.length > 0 || categorization.category === 'harmless') {
          const removes = currentHunk.filter((e) => e.type === 'remove');
          const adds = currentHunk.filter((e) => e.type === 'add');
          const lastRemove =
            removes.length > 0 ? removes[removes.length - 1] : null;
          const lastAdd = adds.length > 0 ? adds[adds.length - 1] : null;
          const endLine1: number = lastRemove?.lineNum1 ?? hunkStartLine1 ?? 0;
          const endLine2: number = lastAdd?.lineNum2 ?? hunkStartLine2 ?? 0;

          const context = extractContext(diff, hunkStartIndex, hunkEndIndex);

          const jsonHunk: JsonHunk = {
            id: `hunk-${hunkId++}`,
            startLine1: hunkStartLine1 || 0,
            endLine1,
            startLine2: hunkStartLine2 || 0,
            endLine2,
            category: categorization.category,
            harmlessType: categorization.harmlessType,
            confidence: categorization.confidence,
            context,
            changes: filtered.map((e) => ({
              type: e.type as 'remove' | 'add',
              line: e.line,
              lineNum: (e.type === 'remove' ? e.lineNum1 : e.lineNum2) || 0,
            })),
            normalized: getNormalizedHunk(
              currentHunk,
              categorization.harmlessType,
            ),
            analysis:
              categorization.category === 'meaningful'
                ? analyzeMeaningfulHunk(currentHunk)
                : undefined,
          };

          hunks.push(jsonHunk);
        }

        currentHunk = [];
        hunkStartLine1 = null;
        hunkStartLine2 = null;
      }
    } else {
      if (currentHunk.length === 0) {
        hunkStartIndex = i;
      }
      if (hunkStartLine1 === null && entry.lineNum1) {
        hunkStartLine1 = entry.lineNum1;
      }
      if (hunkStartLine2 === null && entry.lineNum2) {
        hunkStartLine2 = entry.lineNum2;
      }
      currentHunk.push(entry);
    }
  }

  // Handle remaining hunk at the end
  if (currentHunk.length > 0) {
    const hunkEndIndex = diff.length - 1;
    const categorization = categorizeHunk(
      currentHunk,
      context.ignoreAssetIds,
      context.ignoreUnminifiedRefs,
      context.ignoreSourceMapUrl,
      context.ignoreSwappedVariables,
    );

    const {filtered} = filterHunkEntries(
      currentHunk,
      context.ignoreAssetIds,
      context.ignoreUnminifiedRefs,
      context.ignoreSourceMapUrl,
      context.ignoreSwappedVariables,
    );

    if (filtered.length > 0 || categorization.category === 'harmless') {
      const removes = currentHunk.filter((e) => e.type === 'remove');
      const adds = currentHunk.filter((e) => e.type === 'add');
      const lastRemove =
        removes.length > 0 ? removes[removes.length - 1] : null;
      const lastAdd = adds.length > 0 ? adds[adds.length - 1] : null;
      const endLine1: number = lastRemove?.lineNum1 ?? hunkStartLine1 ?? 0;
      const endLine2: number = lastAdd?.lineNum2 ?? hunkStartLine2 ?? 0;

      const context = extractContext(diff, hunkStartIndex, hunkEndIndex);

      const jsonHunk: JsonHunk = {
        id: `hunk-${hunkId++}`,
        startLine1: hunkStartLine1 || 0,
        endLine1,
        startLine2: hunkStartLine2 || 0,
        endLine2,
        category: categorization.category,
        harmlessType: categorization.harmlessType,
        confidence: categorization.confidence,
        context,
        changes: filtered.map((e) => ({
          type: e.type as 'remove' | 'add',
          line: e.line,
          lineNum: (e.type === 'remove' ? e.lineNum1 : e.lineNum2) || 0,
        })),
        normalized: getNormalizedHunk(currentHunk, categorization.harmlessType),
        analysis:
          categorization.category === 'meaningful'
            ? analyzeMeaningfulHunk(currentHunk)
            : undefined,
      };

      hunks.push(jsonHunk);
    }
  }

  const meaningfulHunks = hunks.filter((h) => h.category === 'meaningful');
  const harmlessHunks = hunks.filter((h) => h.category === 'harmless');

  return {
    path: path.relative(process.cwd(), file1),
    status: hunks.length > 0 ? 'different' : 'identical',
    hunks,
    hunkCount: hunks.length,
    meaningfulHunkCount: meaningfulHunks.length,
    harmlessHunkCount: harmlessHunks.length,
  };
}

/**
 * Generates JSON report for file comparison
 */
export function generateFileJsonReport(
  diff: DiffEntry[],
  file1: string,
  file2: string,
  context: ComparisonContext,
): JsonReport {
  const fileResult = diffToJson(diff, file1, file2, context);

  const hasChanges = diff.some((e) => e.type !== 'equal');

  return {
    metadata: {
      file1: path.resolve(file1),
      file2: path.resolve(file2),
      comparisonDate: new Date().toISOString(),
      options: {
        ignoreAssetIds: context.ignoreAssetIds,
        ignoreUnminifiedRefs: context.ignoreUnminifiedRefs,
        ignoreSourceMapUrl: context.ignoreSourceMapUrl,
        ignoreSwappedVariables: context.ignoreSwappedVariables,
      },
    },
    summary: {
      totalHunks: fileResult.hunkCount,
      meaningfulHunks: fileResult.meaningfulHunkCount,
      harmlessHunks: fileResult.harmlessHunkCount,
      identical: !hasChanges || fileResult.hunkCount === 0,
    },
    files: [fileResult],
  };
}

/**
 * Generates JSON report for directory comparison
 */
export function generateDirectoryJsonReport(
  matched: MatchedPair[],
  ambiguous: AmbiguousMatch[],
  dir1: string,
  dir2: string,
  context: ComparisonContext,
  fileResults: JsonFileResult[],
): JsonReport {
  const meaningfulHunks = fileResults.reduce(
    (sum, f) => sum + f.meaningfulHunkCount,
    0,
  );
  const harmlessHunks = fileResults.reduce(
    (sum, f) => sum + f.harmlessHunkCount,
    0,
  );
  const totalHunks = fileResults.reduce((sum, f) => sum + f.hunkCount, 0);
  const identicalFiles = fileResults.filter(
    (f) => f.status === 'identical',
  ).length;
  const differentFiles = fileResults.filter(
    (f) => f.status === 'different',
  ).length;

  return {
    metadata: {
      dir1: path.resolve(dir1),
      dir2: path.resolve(dir2),
      comparisonDate: new Date().toISOString(),
      options: {
        ignoreAssetIds: context.ignoreAssetIds,
        ignoreUnminifiedRefs: context.ignoreUnminifiedRefs,
        ignoreSourceMapUrl: context.ignoreSourceMapUrl,
        ignoreSwappedVariables: context.ignoreSwappedVariables,
      },
    },
    summary: {
      totalHunks,
      meaningfulHunks,
      harmlessHunks,
      identical: totalHunks === 0,
      identicalFiles,
      differentFiles,
      totalFiles: matched.length,
    },
    files: fileResults,
    ambiguousMatches:
      ambiguous.length > 0
        ? ambiguous.map((a) => ({
            prefix: a.prefix,
            dirPath: a.dirPath,
            files1: a.files1.map((f) => ({path: f.relativePath, size: f.size})),
            files2: a.files2.map((f) => ({path: f.relativePath, size: f.size})),
          }))
        : undefined,
  };
}
