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
   *
   * @author Matt Jones <mjones4@atlassian.com>
   * @since 2024-05-01
   */
  atlaspackV3: false,

  /**
   * Use node.js implementation of @parcel/watcher watchman backend
   *
   * @author Pedro Tacla Yamada <pyamada@atlassian.com>
   * @since 2024-08-09
   */
  useWatchmanWatcher: false,

  /**
   * Configure runtime to enable retriable dynamic imports
   *
   * @author David Alsh <dalsh@atlassian.com>
   * @since 2024-08-21
   */
  importRetry: false,

  /**
   * Fixes quadratic cache invalidation issue
   *
   * @author Pedro Tacla Yamada <pyamada@atlassian.com>
   * @since 2024-10-21
   */
  fixQuadraticCacheInvalidation: 'OLD' as ConsistencyCheckFeatureFlagValue,

  /**
   * Enables an experimental "conditional bundling" API - this allows the use of `importCond` syntax
   * in order to have (consumer) feature flag driven bundling. This feature is very experimental,
   * and requires server-side support.
   *
   * @author Jake Lane <jlane2@atlassian.com>
   * @since 2024-09-11
   */
  conditionalBundlingApi: false,

  /**
   * Enable VCS mode. Expected values are:
   * - OLD - default value, return watchman result
   * - NEW_AND_CHECK - Return VCS result but still call watchman
   * - NEW: Return VCS result, but don't call watchman
   *
   * @author Celeste Carloni <ccarloni@atlassian.com>
   * @since 2025-02-04
   */
  vcsMode: 'OLD' as ConsistencyCheckFeatureFlagValue,

  /**
   * Refactor cache to:
   * - Split writes into multiple entries
   * - Remove "large file blob" writes
   * - Reduce size of the caches by deduplicating data
   *
   * @author Pedro Tacla Yamada <pyamada@atlassian.com>
   * @since 2025-05-13
   */
  cachePerformanceImprovements: process.env.ATLASPACK_BUILD_ENV === 'test',

  /**
   * Deduplicates environments across cache / memory entities
   *
   * @author Pedro Tacla Yamada <pyamada@atlassian.com>
   * @since 2025-06-11
   */
  environmentDeduplication: false,

  /**
   * Enable scanning for the presence of loadable to determine side effects
   *
   * @author Ben Jervis <bjervis@atlassian.com>
   * @since 2025-03-07
   */
  loadableSideEffects: false,

  /**
   * Fixes source maps for inline bundles
   *
   * @author Pedro Tacla Yamada <pyamada@atlassian.com>
   * @since 2025-04-08
   */
  inlineBundlesSourceMapFixes: false,

  /** Enable patch project paths. This will patch the project paths to be relative to the project root.
   * This feature is experimental and should not be used in production. It will used to test downloadble cache artefacts.
   *
   * @author Celeste Carloni <ccarloni@atlassian.com>
   * @since 2025-04-10
   */
  patchProjectPaths: false,

  /**
   * Enable resolution of bundler config starting from the CWD
   *
   * @author Ben Jervis <bjervis@atlassian.com>
   * @since 2025-05-29
   */
  resolveBundlerConfigFromCwd: false,

  /**
   * Enable a setting that allows for more assets to be scope hoisted, if
   * they're safe to do so.
   *
   * @author Ben Jervis <bjervis@atlassian.com>
   * @since 2025-06-17
   */
  applyScopeHoistingImprovement: false,

  /**
   * Enable a change where a constant module only have the namespacing object added in bundles where it is required
   *
   * @author Marcin Szczepanski <mszczepanski@atlassian.com>
   * @since 2025-06-19
   */
  inlineConstOptimisationFix: false,

  /**
   * Improves/fixes HMR behaviour by:
   * - Fixing HMR behaviour with lazy bundle edges
   * - Moving the functionality of the react-refresh runtime into the react-refresh-wrap transformer
   *
   * @author Marcin Szczepanski <mszczepanski@atlassian.com>
   * @since 2025-06-20
   */
  hmrImprovements: false,

  /**
   * Enables the new packaging progress CLI experience
   *
   * @author Matt Jones <mjones4@atlassian.com>
   * @since 2025-07-02
   */
  cliProgressReportingImprovements: false,

  /**
   * Adds support for `webpackChunkName` comments in dynamic imports.
   * Imports with the same `webpackChunkName` will be bundled together.
   *
   * @author Ben Jervis <bjervis@atlassian.com>
   * @since 2025-07-08
   */
  supportWebpackChunkName: process.env.ATLASPACK_BUILD_ENV === 'test',

  /**
   * Enable a change to the conditional bundling loader to use a fallback bundle loading if the expected scripts aren't found
   *
   * Split into two flags, to allow usage in the dev or prod packagers separately
   *
   * @author Jake Lane <jlane2@atlassian.com>
   * @since 2025-07-08
   */
  condbDevFallbackDev: false,
  /**
   *
   * @author Jake Lane <jlane2@atlassian.com>
   * @since 2025-07-08
   */
  condbDevFallbackProd: false,

  /**
   * Enable the new incremental bundling versioning logic which determines whether
   * a full bundling pass is required based on the AssetGraph's bundlingVersion.
   *
   * @author Pedro Tacla Yamada <pyamada@atlassian.com>
   * @since 2025-07-08
   */
  incrementalBundlingVersioning: process.env.ATLASPACK_BUILD_ENV === 'test',

  /**
   * Remove redundant shared bundles that are no longer required after merging
   * async bundles.
   *
   * @author Matt Jones <mjones4@atlassian.com>
   * @since 2025-08-20
   */
  removeRedundantSharedBundles: process.env.ATLASPACK_BUILD_ENV === 'test',

  /**
   * When enabled, single file output bundles have a stable name
   *
   * @author Marcin Szczepanski <mszczepanski@atlassian.com>
   * @since 2025-08-21
   */
  singleFileOutputStableName: process.env.ATLASPACK_BUILD_ENV === 'test',

  /**
   * Enable optimised prelude for the ScopeHoistingPackager.
   *
   * @author Jake Lane <jlane2@atlassian.com>
   * @since 2025-08-22
   */
  useNewPrelude: false,

  /**
   * Enable a fix for applyScopeHoistingImprovement that allows assets to still
   * be at the top level of the bundle.
   *
   * @author Ben Jervis <bjervis@atlassian.com>
   * @since 2025-08-27
   */
  applyScopeHoistingImprovementV2: false,

  /**
   * When enabled, if both explicit entries and explicit targets are specified,
   * the source properties of those targets are used as filters against the base entries.
   * This allows building only specific entries for specific targets.
   *
   * @author Marcin Szczepanski <mszczepanski@atlassian.com>
   * @since 2025-09-03
   */
  allowExplicitTargetEntries: process.env.ATLASPACK_BUILD_ENV === 'test',
  /**
   * When enabled, the packager will avoid using the binding helper for exports where possible.
   *
   * @author Jake Lane <jlane2@atlassian.com>
   * @since 2025-09-08
   */
  exportsRebindingOptimisation: false,

  /**
   * When enabled, ensures the `unstableSingleFileOutput` environment property is preserved during CSS transformation
   *
   * @author Marcin Szczepanski <mszczepanski@atlassian.com>
   * @since 2025-09-09
   */
  preserveUnstableSingleFileOutputInCss:
    process.env.ATLASPACK_BUILD_ENV === 'test',

  /**
   * When enabled, fixes handling of symbol locations when source maps contain
   * project relative paths
   *
   * @author Matt Jones <mjones4@atlassian.com>
   * @since 2025-09-18
   */
  symbolLocationFix: process.env.ATLASPACK_BUILD_ENV === 'test',

  /**
   * When enabled, avoid retaining sourcesContent in memory during transformation.
   * Source contents will be inlined only during final map stringify if needed.
   * @author Shanon Jackson <sjackson3@atlassian.com>
   * @since 2025-09-22
   */
  omitSourcesContentInMemory: false,

  /**
   * Fixes an issue in BundleGraph.fromAssetGraph where Dependency.sourceAssetId
   * diverging from Asset.id can cause it to fail. The fix to to stop using Dependency.sourceAssetId
   * all together and use graph.getNodeIdsConnectedTo instead.
   *
   * @author Matt Jones <mjones4@atlassian.com>
   * @since 2025-09-29
   */
  sourceAssetIdBundleGraphFix: process.env.ATLASPACK_BUILD_ENV === 'test',
  /**
   * When enabled, deduplicates reporters when resolving the config.
   *
   * @author Vy Kim Nguyen <vnguyen4@atlassian.com>
   * @since 2025-10-14
   */
  deduplicateReporters: process.env.ATLASPACK_BUILD_ENV === 'test',
  /**
   * Enable JSX configuration loading in v3 Rust transformer to match v2 behaviour
   *
   * @author matt-koko <mkokolich@atlassian.com>
   * @since 2025-10-21
   */
  v3JsxConfigurationLoading: process.env.ATLASPACK_BUILD_ENV === 'test',

  /**

   * When _disabled_, will early exit from the @atlaspack/transformer-tokens transformation
   *
   * @author Marcin Szczepanski <mszczepanski@atlassian.com>
   * @since 2025-10-17
   */
  enableTokensTransformer: process.env.ATLASPACK_BUILD_ENV === 'test',

  /*
   * When enabled, applies the SWC compiled CSS in JS transformer to the codebase.
   *
   * This is a temporary feature flag for the migration state. We eventually will remove this transformer plugin and directly use the SWC visitor in the JS transform.
   *
   * @author Jake Lane <jlane2@atlassian.com>
   * @since 2025-10-16
   */
  compiledCssInJsTransformer: process.env.ATLASPACK_BUILD_ENV === 'test',
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
