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
   * Enable Rust based LMDB wrapper library
   */
  useLmdbJsLite: boolean,
  /**
   * Fixes quadratic cache invalidation issue
   */
  fixQuadraticCacheInvalidation: ConsistencyCheckFeatureFlagValue,
  /**
   * Enable rust based inline requires optimization
   */
  fastOptimizeInlineRequires: boolean,
  /**
   * Enables an experimental "conditional bundling" API - this allows the use of `importCond` syntax
   * in order to have (consumer) feature flag driven bundling. This feature is very experimental,
   * and requires server-side support.
   */
  conditionalBundlingApi: boolean,
|};

export type ConsistencyCheckFeatureFlagValue =
  | 'NEW'
  | 'OLD'
  | 'NEW_AND_CHECK'
  | 'OLD_AND_CHECK';