/* eslint-disable monorepo/no-internal-import */

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

export interface PackagingTestOptions {
  cacheDir: string;
  outputDir?: string;
  bundleFilter?: (bundle: any) => boolean;
  verbose?: boolean;
  /** Enable comparison mode: run both native and JS packagers */
  compare?: boolean;
}

// ---------------------------------------------------------------------------
// Single-bundle results
// ---------------------------------------------------------------------------

export interface PackagingResult {
  bundleId: string;
  bundleType: string;
  bundleName: string | null | undefined;
  outputPath: string;
  size: number;
  hash: string;
  timeMs: number;
  cacheKeys: {
    content: string;
    map: string;
    info: string;
  };
}

export interface JSPackagerResult {
  bundleId: string;
  bundleType: string;
  bundleName: string | null | undefined;
  outputPath: string;
  size: number;
  timeMs: number;
  contents: string;
}

export interface ComparisonResult {
  native: PackagingResult;
  js: JSPackagerResult;
  stats: {
    sizeDiff: number;
    sizeDiffPercent: number;
    timeDiff: number;
    timeDiffPercent: number;
    nativeFaster: boolean;
    nativeSmaller: boolean;
  };
}
