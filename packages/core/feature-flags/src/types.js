// @flow strict

export type FeatureFlags = {|
  // This feature flag mostly exists to test the feature flag system, and doesn't have any build/runtime effect
  +exampleFeature: boolean,
  +exampleConsistencyCheckFeature: ConsistencyCheckFeatureFlagValue,
  /**
   * Rust backed requests
   */
  +atlaspackV3: boolean,
  /**
   * Use node.js implementation of @parcel/watcher watchman backend
   */
  +useWatchmanWatcher: boolean,
  /**
   * Configure runtime to enable retriable dynamic imports
   */
  importRetry: boolean,
  /**
   * Fixes quadratic cache invalidation issue
   */
  fixQuadraticCacheInvalidation: ConsistencyCheckFeatureFlagValue,
  /**
   * Enables an experimental "conditional bundling" API - this allows the use of `importCond` syntax
   * in order to have (consumer) feature flag driven bundling. This feature is very experimental,
   * and requires server-side support.
   */
  conditionalBundlingApi: boolean,
  /**
   * Run inline requires optimizer in the rayon thread pool.
   */
  inlineRequiresMultiThreading: boolean,
  /**
   * Disables aborting of builds and fixes bugs related to state corruption on abort.
   */
  fixBuildAbortCorruption: boolean,
  /**
   * Enable VCS mode. Expected values are:
   * - OLD - default value, return watchman result
   * - NEW_AND_CHECK - Return VCS result but still call watchman
   * - NEW: Return VCS result, but don't call watchman
   */
  vcsMode: ConsistencyCheckFeatureFlagValue,
  /**
   * Refactor cache to:
   * - Split writes into multiple entries
   * - Remove "large file blob" writes
   * - Reduce size of the caches by deduplicating data
   */
  cachePerformanceImprovements: boolean,
  /**
   * Deduplicates environments across cache / memory entities
   */
  environmentDeduplication: boolean,
  /**
   * Enable scanning for the presence of loadable to determine side effects
   */
  loadableSideEffects: boolean,
  /**
   * Enable performance optimization for the resolver specifier to_string
   * conversions
   */
  reduceResolverStringCreation: boolean,
  /**
   * Add verbose metrics for request tracker invalidation
   */
  verboseRequestInvalidationStats: boolean,
  /**
   * Fixes source maps for inline bundles
   */
  inlineBundlesSourceMapFixes: boolean,
  /** Enable patch project paths. This will patch the project paths to be relative to the project root.
   * This feature is experimental and should not be used in production. It will used to test downloadble cache artefacts.
   */
  patchProjectPaths: boolean,
  /**
   * Enables optimized inline string replacement perf for the packager.
   * Used heavily for inline bundles.
   */
  inlineStringReplacementPerf: boolean,
  /**
   * Enable resolution of bundler config starting from the CWD
   */
  resolveBundlerConfigFromCwd: boolean,
  /**
   * Enable a setting that allows for more assets to be scope hoisted, if
   * they're safe to do so.
   */
  applyScopeHoistingImprovement: boolean,
  /**
   * Enable a change where a constant module only have the namespacing object added in bundles where it is required
   */
  inlineConstOptimisationFix: boolean,
  /**
   * Improves/fixes HMR behaviour by:
   * - Fixing HMR behaviour with lazy bundle edges
   * - Moving the functionality of the react-refresh runtime into the react-refresh-wrap transformer
   */
  hmrImprovements: boolean,
  /**
   * Adds an end() method to AtlaspckV3 to cleanly shutdown the NAPI worker pool
   */
  atlaspackV3CleanShutdown: boolean,
  /**
   * Fixes a bug where imported objects that are accessed with non-static
   * properties (e.g. `CONSTANTS['api_' + endpoint`]) would not be recognised as
   * being used, and thus not included in the bundle.
   */
  unusedComputedPropertyFix: boolean,

  /**
   * Fixes an issue where star re-exports of empty files (usually occuring in compiled typescript libraries)
   * could cause exports to undefined at runtime.
   */
  emptyFileStarRexportFix: boolean,
|};

declare export var CONSISTENCY_CHECK_VALUES: $ReadOnlyArray<string>;
export type ConsistencyCheckFeatureFlagValue = $ElementType<
  typeof CONSISTENCY_CHECK_VALUES,
  number,
>;

declare export var DEFAULT_FEATURE_FLAGS: FeatureFlags;
