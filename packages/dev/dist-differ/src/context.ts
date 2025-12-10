/**
 * Context object that holds all comparison options and paths
 * This simplifies function signatures by avoiding many individual parameters
 */
export interface ComparisonContext {
  // Paths being compared
  file1?: string;
  file2?: string;
  dir1?: string;
  dir2?: string;

  // Ignore options
  ignoreAssetIds: boolean;
  ignoreUnminifiedRefs: boolean;
  ignoreSourceMapUrl: boolean;
  ignoreSwappedVariables: boolean;

  // Output options
  summaryMode: boolean;
  verbose: boolean;
  jsonMode: boolean;

  // Matching options
  sizeThreshold: number;
}

/**
 * Creates a context object from CLI options and paths
 */
export function createContext(
  file1?: string,
  file2?: string,
  dir1?: string,
  dir2?: string,
  options?: {
    ignoreAssetIds?: boolean;
    ignoreUnminifiedRefs?: boolean;
    ignoreSourceMapUrl?: boolean;
    ignoreSwappedVariables?: boolean;
    summaryMode?: boolean;
    verbose?: boolean;
    jsonMode?: boolean;
    sizeThreshold?: number;
  },
): ComparisonContext {
  return {
    file1,
    file2,
    dir1,
    dir2,
    ignoreAssetIds: options?.ignoreAssetIds ?? false,
    ignoreUnminifiedRefs: options?.ignoreUnminifiedRefs ?? false,
    ignoreSourceMapUrl: options?.ignoreSourceMapUrl ?? false,
    ignoreSwappedVariables: options?.ignoreSwappedVariables ?? false,
    summaryMode: options?.summaryMode ?? false,
    verbose: options?.verbose ?? false,
    jsonMode: options?.jsonMode ?? false,
    sizeThreshold: options?.sizeThreshold ?? 0.01,
  };
}
