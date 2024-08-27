// @flow strict

export type FeatureFlags = {|
  // This feature flag mostly exists to test the feature flag system, and doesn't have any build/runtime effect
  +exampleFeature: boolean,
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
   * Enable fast path for needsDefaultInterop.
   *
   * This improves bundling performance of large applications very significantly.
   */
  fastNeedsDefaultInterop: boolean,
  /**
   * Enable resolver refactor into owned data structures.
   */
  ownedResolverStructures: boolean,
|};
