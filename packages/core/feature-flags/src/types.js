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
   * Enable scanning for the presence of loadable to determine side effects
   */
  loadableSideEffects: boolean,
  /**
   * Enable performance optimization for the resolver specifier to_string
   * conversions
   */
  reduceResolverStringCreation: boolean,
  /**
   * Fixes source maps for inline bundles
   */
  inlineBundlesSourceMapFixes: boolean,
  /**
   * Enable nested loading of bundles in the runtime with conditional bundling
   */
  conditionalBundlingNestedRuntime: boolean,
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
   * Enable support for the async bundle runtime (unstable_asyncBundleRuntime) in conditional bundling
   */
  conditionalBundlingAsyncRuntime: boolean,
  /**
   * Fix a bug where the conditional manifest reporter would report and write the same manifest multiple times
   */
  conditionalBundlingReporterDuplicateFix: boolean,
|};

export type ConsistencyCheckFeatureFlagValue =
  | 'NEW'
  | 'OLD'
  | 'NEW_AND_CHECK'
  | 'OLD_AND_CHECK';
