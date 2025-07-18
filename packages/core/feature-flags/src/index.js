// @flow strict

import type {FeatureFlags as _FeatureFlags} from './types';
// We need to do these gymnastics as we don't want flow-to-ts to touch DEFAULT_FEATURE_FLAGS,
// but we want to export FeatureFlags for Flow
export type FeatureFlags = _FeatureFlags;

export const CONSISTENCY_CHECK_VALUES: $ReadOnlyArray<string> = Object.freeze([
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
  condbDevFallbackDev: false,
  condbDevFallbackProd: false,
  incrementalBundlingVersioning: process.env.NODE_ENV === 'test',
};

let featureFlagValues: FeatureFlags = {...DEFAULT_FEATURE_FLAGS};

export function setFeatureFlags(flags: FeatureFlags) {
  featureFlagValues = flags;
}

export function getFeatureFlag(flagName: $Keys<FeatureFlags>): boolean {
  const value = featureFlagValues[flagName];
  return value === true || value === 'NEW';
}

export function getFeatureFlagValue(
  flagName: $Keys<FeatureFlags>,
): boolean | string | number {
  return featureFlagValues[flagName];
}

export type DiffResult<CustomDiagnostic> = {|
  isDifferent: boolean,
  custom: CustomDiagnostic,
|};

export type Diagnostic<CustomDiagnostic> = {|
  isDifferent: boolean,
  oldExecutionTimeMs: number,
  newExecutionTimeMs: number,
  custom: CustomDiagnostic,
|};

/**
 * Run a function with a consistency check.
 */
export function runWithConsistencyCheck<Result, CustomDiagnostic>(
  flag: string,
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
