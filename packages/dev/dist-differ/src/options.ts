import type {CliOptions} from './cli';

/**
 * Default options for diff operations
 */
export const DEFAULT_OPTIONS: CliOptions = {
  ignoreAssetIds: false,
  ignoreUnminifiedRefs: false,
  ignoreSourceMapUrl: false,
  ignoreSwappedVariables: false,
  summaryMode: false,
  verbose: false,
  jsonMode: false,
  sizeThreshold: 0.01, // Default 1%
};

/**
 * Validates size threshold value
 */
export function validateSizeThreshold(value: number): boolean {
  return !isNaN(value) && value >= 0 && value <= 1;
}
