// Converted from Flow to TypeScript

export type FeatureFlags = {
  /**
   * This feature flag mostly exists to test the feature flag system, and doesn't have any build/runtime effect
   */
  readonly exampleFeature: boolean;
  readonly exampleConsistencyCheckFeature: ConsistencyCheckFeatureFlagValue;
  /**
   * Rust backed requests
   */
  readonly atlaspackV3: boolean;
  /**
   * Use node.js implementation of @parcel/watcher watchman backend
   */
  readonly useWatchmanWatcher: boolean;
  /**
   * Configure runtime to enable retriable dynamic imports
   */
  importRetry: boolean;
  /**
   * Fixes quadratic cache invalidation issue
   */
  fixQuadraticCacheInvalidation: ConsistencyCheckFeatureFlagValue;
  /**
   * Enables an experimental "conditional bundling" API - this allows the use of `importCond` syntax
   * in order to have (consumer) feature flag driven bundling. This feature is very experimental,
   * and requires server-side support.
   */
  conditionalBundlingApi: boolean;
  /**
   * Enable VCS mode. Expected values are:
   * - OLD - default value, return watchman result
   * - NEW_AND_CHECK - Return VCS result but still call watchman
   * - NEW: Return VCS result, but don't call watchman
   */
  vcsMode: ConsistencyCheckFeatureFlagValue;
  /**
   * Refactor cache to:
   * - Split writes into multiple entries
   * - Remove "large file blob" writes
   * - Reduce size of the caches by deduplicating data
   */
  cachePerformanceImprovements: boolean;
  /**
   * Deduplicates environments across cache / memory entities
   */
  environmentDeduplication: boolean;
  /**
   * Enable scanning for the presence of loadable to determine side effects
   */
  loadableSideEffects: boolean;
  /**
   * Enable performance optimization for the resolver specifier to_string
   * conversions
   */
  reduceResolverStringCreation: boolean;
  /**
   * Add verbose metrics for request tracker invalidation
   */
  verboseRequestInvalidationStats: boolean;
  /**
   * Fixes source maps for inline bundles
   */
  inlineBundlesSourceMapFixes: boolean;
  /** Enable patch project paths. This will patch the project paths to be relative to the project root.
   * This feature is experimental and should not be used in production. It will used to test downloadble cache artefacts.
   */
  patchProjectPaths: boolean;
  /**
   * Enables optimized inline string replacement perf for the packager.
   * Used heavily for inline bundles.
   */
  inlineStringReplacementPerf: boolean;
  /**
   * Enable resolution of bundler config starting from the CWD
   */
  resolveBundlerConfigFromCwd: boolean;
  /**
   * Enable a setting that allows for more assets to be scope hoisted, if
   * they're safe to do so.
   */
  applyScopeHoistingImprovement: boolean;
  /**
   * Enable a change where a constant module only have the namespacing object added in bundles where it is required
   */
  inlineConstOptimisationFix: boolean;
  /**
   * Improves/fixes HMR behaviour by:
   * - Fixing HMR behaviour with lazy bundle edges
   * - Moving the functionality of the react-refresh runtime into the react-refresh-wrap transformer
   */
  hmrImprovements: boolean;
  /**
   * Adds an end() method to AtlaspckV3 to cleanly shutdown the NAPI worker pool
   */
  atlaspackV3CleanShutdown: boolean;
  /**
   * Fixes a bug where imported objects that are accessed with non-static
   * properties (e.g. `CONSTANTS['api_' + endpoint`]) would not be recognised as
   * being used, and thus not included in the bundle.
   */
  unusedComputedPropertyFix: boolean;
  /**
   * Fixes an issue where star re-exports of empty files (usually occurring in compiled typescript libraries)
   * could cause exports to undefined at runtime.
   */
  emptyFileStarRexportFix: boolean;
  /**
   * Enables the new packaging progress CLI experience
   */
  cliProgressReportingImprovements: boolean;
  /**
   * Enable a change to the conditional bundling loader to use a fallback bundle loading if the expected scripts aren't found
   *
   * Split into two flags, to allow usage in the dev or prod packagers separately
   */
  condbDevFallbackDev: boolean;
  condbDevFallbackProd: boolean;
  /**
   * Enable the new incremental bundling versioning logic which determines whether
   * a full bundling pass is required based on the AssetGraph's bundlingVersion.
   */
  incrementalBundlingVersioning: boolean;
};

export type ConsistencyCheckFeatureFlagValue =
  (typeof CONSISTENCY_CHECK_VALUES)[number];

export const CONSISTENCY_CHECK_VALUES: ReadonlyArray<string> = Object.freeze([
  'NEW',
  'OLD',
  'NEW_AND_CHECK',
  'OLD_AND_CHECK',
]);

export const DEFAULT_FEATURE_FLAGS: FeatureFlags = {
  exampleConsistencyCheckFeature: 'OLD',
  exampleFeature: false,
  atlaspackV3: false,
  useWatchmanWatcher: false,
  importRetry: false,
  fixQuadraticCacheInvalidation: 'OLD',
  conditionalBundlingApi: false,
  vcsMode: 'OLD',
  loadableSideEffects: false,
  reduceResolverStringCreation: false,
  inlineBundlesSourceMapFixes: false,
  patchProjectPaths: false,
  cachePerformanceImprovements: process.env.NODE_ENV === 'test',
  environmentDeduplication: false,
  inlineStringReplacementPerf: false,
  // Default to true as it's a monitoring change. Can be turned off if necessary.
  verboseRequestInvalidationStats: true,
  resolveBundlerConfigFromCwd: false,
  applyScopeHoistingImprovement: false,
  inlineConstOptimisationFix: false,
  hmrImprovements: false,
  atlaspackV3CleanShutdown: false,
  unusedComputedPropertyFix: process.env.NODE_ENV === 'test',
  emptyFileStarRexportFix: process.env.NODE_ENV === 'test',
  cliProgressReportingImprovements: false,
  supportWebpackChunkName: process.env.NODE_ENV === 'test',
  condbDevFallbackDev: false,
  condbDevFallbackProd: false,
  incrementalBundlingVersioning: process.env.NODE_ENV === 'test',
};

let featureFlagValues: FeatureFlags = {...DEFAULT_FEATURE_FLAGS};

export function setFeatureFlags(flags: FeatureFlags) {
  featureFlagValues = flags;
}

export function getFeatureFlag(flagName: keyof FeatureFlags): boolean {
  const value = featureFlagValues[flagName];
  return value === true || value === 'NEW';
}

export function getFeatureFlagValue(
  flagName: keyof FeatureFlags,
): boolean | string | number {
  return featureFlagValues[flagName];
}

export type DiffResult<CustomDiagnostic> = {
  isDifferent: boolean;
  custom: CustomDiagnostic;
};

export type Diagnostic<CustomDiagnostic> = {
  isDifferent: boolean;
  oldExecutionTimeMs: number;
  newExecutionTimeMs: number;
  custom: CustomDiagnostic;
};

/**
 * Run a function with a consistency check.
 */
export function runWithConsistencyCheck<Result, CustomDiagnostic>(
  flag: keyof FeatureFlags,
  oldFn: () => Result,
  newFn: () => Result,
  diffFn: (
    oldResult: Result,
    newResult: Result,
  ) => DiffResult<CustomDiagnostic>,
  report: (
    diagnostic: Diagnostic<CustomDiagnostic>,
    oldResult: Result,
    newResult: Result,
  ) => void,
): Result {
  const value = featureFlagValues[flag];
  // @ts-expect-error - TypeScript doesn't understand the union type comparison
  if (!value || value === false || value === 'OLD') {
    return oldFn();
  }
  if (value === true || value === 'NEW') {
    return newFn();
  }

  const oldStartTime = performance.now();
  const oldResult = oldFn();
  const oldExecutionTimeMs = performance.now() - oldStartTime;

  const newStartTime = performance.now();
  const newResult = newFn();
  const newExecutionTimeMs = performance.now() - newStartTime;

  const diff = diffFn(oldResult, newResult);

  report(
    {
      isDifferent: diff.isDifferent,
      oldExecutionTimeMs,
      newExecutionTimeMs,
      custom: diff.custom,
    },
    oldResult,
    newResult,
  );

  if (value === 'NEW_AND_CHECK') {
    return newResult;
  }

  return oldResult;
}
