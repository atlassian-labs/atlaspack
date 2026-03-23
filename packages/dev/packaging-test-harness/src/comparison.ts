/* eslint-disable monorepo/no-internal-import */
import type {
  ComparisonResult,
  JSPackagerResult,
  PackagingResult,
} from './types';

/**
 * Format a byte count to a human-readable string.
 */
export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(2)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

/**
 * Calculate comparison statistics between a native and JS single-bundle result.
 */
export function calculateComparisonStats(
  native: PackagingResult,
  js: JSPackagerResult,
): ComparisonResult['stats'] {
  const sizeDiff = native.size - js.size;
  const sizeDiffPercent = js.size > 0 ? (sizeDiff / js.size) * 100 : 0;
  const timeDiff = native.timeMs - js.timeMs;
  const timeDiffPercent = js.timeMs > 0 ? (timeDiff / js.timeMs) * 100 : 0;
  return {
    sizeDiff,
    sizeDiffPercent,
    timeDiff,
    timeDiffPercent,
    nativeFaster: native.timeMs < js.timeMs,
    nativeSmaller: native.size < js.size,
  };
}

/**
 * Format single-bundle comparison results for display.
 */
export function formatComparisonResults(comparison: ComparisonResult): string {
  const {native, js, stats} = comparison;
  return [
    '',
    '='.repeat(60),
    'COMPARISON RESULTS',
    '='.repeat(60),
    '',
    'Native Packager (Rust):',
    `  Size: ${formatSize(native.size)}`,
    `  Time: ${native.timeMs.toFixed(2)}ms`,
    '',
    'JS Packager:',
    `  Size: ${formatSize(js.size)}`,
    `  Time: ${js.timeMs.toFixed(2)}ms`,
    '',
    '-'.repeat(60),
    'Differences:',
    `  Size: ${stats.sizeDiff >= 0 ? '+' : ''}${formatSize(stats.sizeDiff)} (${stats.sizeDiffPercent >= 0 ? '+' : ''}${stats.sizeDiffPercent.toFixed(2)}%)`,
    `  Time: ${stats.timeDiff >= 0 ? '+' : ''}${stats.timeDiff.toFixed(2)}ms (${stats.timeDiffPercent >= 0 ? '+' : ''}${stats.timeDiffPercent.toFixed(2)}%)`,
    '',
    'Summary:',
    `  ${stats.nativeFaster ? 'Native is FASTER' : 'JS is FASTER'} by ${Math.abs(stats.timeDiff).toFixed(2)}ms`,
    `  ${stats.nativeSmaller ? 'Native is SMALLER' : 'JS is SMALLER'} by ${formatSize(Math.abs(stats.sizeDiff))}`,
    '='.repeat(60),
  ].join('\n');
}

/**
 * Type guard: result is a single-bundle ComparisonResult.
 */
export function isComparisonResult(
  result: PackagingResult | ComparisonResult,
): result is ComparisonResult {
  return 'native' in result && 'js' in result && 'stats' in result;
}
