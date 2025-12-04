/**
 * Normalization patterns and functions for comparing minified code
 * Handles asset IDs and unminified refs that may differ between builds
 */

// Asset ID pattern: quoted 5-character base62 strings (0-9, a-z, A-Z)
const ASSET_ID_QUOTED_PATTERN = /"[0-9a-zA-Z]{5}"/g;
// Variable name pattern: $ followed by 5-character base62 string
const ASSET_ID_VAR_PATTERN = /\$[0-9a-zA-Z]{5}/g;
const ASSET_ID_QUOTED_PLACEHOLDER = '"ASSET_ID"';
const ASSET_ID_VAR_PLACEHOLDER = '$ASSET_ID';

// Unminified ref patterns: $ followed by 16 hex digits, then $exports or $var$...
// We'll match and replace just the hex part, keeping everything else intact
const UNMINIFIED_REF_EXPORTS_HEX_PATTERN = /[$][0-9a-fA-F]{16}[$](?=exports)/g;
const UNMINIFIED_REF_VAR_HEX_PATTERN = /[$][0-9a-fA-F]{16}[$](?=var[$])/g;

/**
 * Normalizes asset IDs in a line by replacing them with placeholders
 */
export function normalizeAssetIds(line: string): string {
  return line
    .replace(ASSET_ID_QUOTED_PATTERN, ASSET_ID_QUOTED_PLACEHOLDER)
    .replace(ASSET_ID_VAR_PATTERN, ASSET_ID_VAR_PLACEHOLDER);
}

/**
 * Normalizes unminified refs in a line by replacing hex parts with placeholders
 */
export function normalizeUnminifiedRefs(line: string): string {
  // Reset regex lastIndex to avoid state issues with global flag
  UNMINIFIED_REF_EXPORTS_HEX_PATTERN.lastIndex = 0;
  UNMINIFIED_REF_VAR_HEX_PATTERN.lastIndex = 0;
  return line
    .replace(UNMINIFIED_REF_EXPORTS_HEX_PATTERN, '$UNMINIFIED_REF$')
    .replace(UNMINIFIED_REF_VAR_HEX_PATTERN, '$UNMINIFIED_REF$');
}

/**
 * Checks if two lines differ only by asset IDs
 */
export function linesDifferOnlyByAssetIds(
  line1: string,
  line2: string,
): boolean {
  return normalizeAssetIds(line1) === normalizeAssetIds(line2);
}

/**
 * Checks if two lines differ only by unminified refs
 */
export function linesDifferOnlyByUnminifiedRefs(
  line1: string,
  line2: string,
): boolean {
  return normalizeUnminifiedRefs(line1) === normalizeUnminifiedRefs(line2);
}
