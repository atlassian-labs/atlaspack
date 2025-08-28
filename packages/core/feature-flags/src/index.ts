// Converted from Flow to TypeScript

export type ConsistencyCheckFeatureFlagValue =
  (typeof CONSISTENCY_CHECK_VALUES)[number];

export const CONSISTENCY_CHECK_VALUES: ReadonlyArray<string> = Object.freeze([
  'NEW',
  'OLD',
  'NEW_AND_CHECK',
  'OLD_AND_CHECK',
]);

export const DEFAULT_FEATURE_FLAGS = {
  // This feature flag mostly exists to test the feature flag system, and doesn't have any build/runtime effect
  exampleFeature: false,
  exampleConsistencyCheckFeature: 'OLD' as ConsistencyCheckFeatureFlagValue,

  /**
   * Rust backed requests
   */
  atlaspackV3: false,

  /**
   * Use node.js implementation of @parcel/watcher watchman backend
   */
  useWatchmanWatcher: false,

  /**
   * Configure runtime to enable retriable dynamic imports
   */
  importRetry: false,

  /**
   * Fixes quadratic cache invalidation issue
   */
  fixQuadraticCacheInvalidation: 'OLD' as ConsistencyCheckFeatureFlagValue,

  /**
   * Enables an experimental "conditional bundling" API - this allows the use of `importCond` syntax
   * in order to have (consumer) feature flag driven bundling. This feature is very experimental,
   * and requires server-side support.
   */
  conditionalBundlingApi: false,

  /**
   * Enable VCS mode. Expected values are:
   * - OLD - default value, return watchman result
   * - NEW_AND_CHECK - Return VCS result but still call watchman
   * - NEW: Return VCS result, but don't call watchman
   */
  vcsMode: 'OLD' as ConsistencyCheckFeatureFlagValue,

  /**
   * Refactor cache to:
   * - Split writes into multiple entries
   * - Remove "large file blob" writes
   * - Reduce size of the caches by deduplicating data
   */
  cachePerformanceImprovements: process.env.ATLASPACK_BUILD_ENV === 'test',

  /**
   * Deduplicates environments across cache / memory entities
   */
  environmentDeduplication: false,

  /**
   * Enable scanning for the presence of loadable to determine side effects
   */
  loadableSideEffects: false,

  /**
   * Enable performance optimization for the resolver specifier to_string
   * conversions
   */
  reduceResolverStringCreation: false,

  /**
   * Add verbose metrics for request tracker invalidation.
   * Default to true as it's a monitoring change. Can be turned off if necessary.
   */
  verboseRequestInvalidationStats: true,

  /**
   * Fixes source maps for inline bundles
   */
  inlineBundlesSourceMapFixes: false,

  /** Enable patch project paths. This will patch the project paths to be relative to the project root.
   * This feature is experimental and should not be used in production. It will used to test downloadble cache artefacts.
   */
  patchProjectPaths: false,

  /**
   * Enables optimized inline string replacement perf for the packager.
   * Used heavily for inline bundles.
   */
  inlineStringReplacementPerf: false,

  /**
   * Enable resolution of bundler config starting from the CWD
   */
  resolveBundlerConfigFromCwd: false,

  /**
   * Enable a setting that allows for more assets to be scope hoisted, if
   * they're safe to do so.
   */
  applyScopeHoistingImprovement: false,

  /**
   * Enable a change where a constant module only have the namespacing object added in bundles where it is required
   */
  inlineConstOptimisationFix: false,

  /**
   * Improves/fixes HMR behaviour by:
   * - Fixing HMR behaviour with lazy bundle edges
   * - Moving the functionality of the react-refresh runtime into the react-refresh-wrap transformer
   */
  hmrImprovements: false,

  /**
   * Fixes a bug where imported objects that are accessed with non-static
   * properties (e.g. `CONSTANTS['api_' + endpoint`]) would not be recognised as
   * being used, and thus not included in the bundle.
   */
  unusedComputedPropertyFix: process.env.ATLASPACK_BUILD_ENV === 'test',

  /**
   * Fixes an issue where star re-exports of empty files (usually occurring in compiled typescript libraries)
   * could cause exports to undefined at runtime.
   */
  emptyFileStarRexportFix: process.env.ATLASPACK_BUILD_ENV === 'test',

  /**
   * Enables the new packaging progress CLI experience
   */
  cliProgressReportingImprovements: false,

  /**
   * Adds support for `webpackChunkName` comments in dynamic imports.
   * Imports with the same `webpackChunkName` will be bundled together.
   */
  supportWebpackChunkName: process.env.ATLASPACK_BUILD_ENV === 'test',

  /**
   * Enable a change to the conditional bundling loader to use a fallback bundle loading if the expected scripts aren't found
   *
   * Split into two flags, to allow usage in the dev or prod packagers separately
   */
  condbDevFallbackDev: false,
  condbDevFallbackProd: false,

  /**
   * Enable the new incremental bundling versioning logic which determines whether
   * a full bundling pass is required based on the AssetGraph's bundlingVersion.
   */
  incrementalBundlingVersioning: process.env.ATLASPACK_BUILD_ENV === 'test',

  /**
   * Remove redundant shared bundles that are no longer required after merging
   * async bundles.
   */
  removeRedundantSharedBundles: process.env.ATLASPACK_BUILD_ENV === 'test',

  /**
   * When enabled, single file output bundles have a stable name
   */
  singleFileOutputStableName: process.env.ATLASPACK_BUILD_ENV === 'test',

  /**
   * Enable optimised prelude for the ScopeHoistingPackager.
   */
  useNewPrelude: false,

  /**
   * Enable a fix for applyScopeHoistingImprovement that allows assets to still
   * be at the top level of the bundle.
   */
  applyScopeHoistingImprovementV2: false,

  /**
   * When enabled, if both explicit entries and explicit targets are specified,
   * the source properties of those targets are used as filters against the base entries.
   * This allows building only specific entries for specific targets.
   */
  allowExplicitTargetEntries: process.env.ATLASPACK_BUILD_ENV === 'test',

  /**
   * When enabled, allows custom per-target "env" properties to be used in transformers.
   */
  customEnvInTargets: false,
};

export type FeatureFlags = typeof DEFAULT_FEATURE_FLAGS;

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
  if (value === false || value === '' || value === 'OLD') {
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
