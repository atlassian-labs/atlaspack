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

// Source map URL pattern: //# sourceMappingURL=... (can be //# or //@)
// Matches the source map URL comment and everything after it, including optional trailing newlines
const SOURCE_MAP_URL_PATTERN = /\/\/[#@]\s*sourceMappingURL=[^\n]*(?:\n|$)/;
const SOURCE_MAP_URL_PLACEHOLDER = '//# sourceMappingURL=SOURCE_MAP_URL';

/**
 * List of global objects that should never be transformed into scoped variables
 * These are JavaScript built-in globals that must remain as direct references
 */
const GLOBAL_OBJECTS = new Set([
  'globalThis',
  'window',
  'self',
  'document',
  'navigator',
  'console',
  'process',
  'Buffer',
  'global',
]);

/**
 * Pattern to match scoped variable references: $<hex>$var$<identifier>
 * Note: We use [$] to match literal $ (in character class, $ doesn't need escaping)
 */
const SCOPED_VAR_PATTERN = /[$][0-9a-fA-F]{16}[$]var[$]/g;

/**
 * Checks if a line contains a transformation of a global object into a scoped variable
 * This is a breaking change and should never be considered harmless
 */
export function hasGlobalObjectTransformation(line: string): boolean {
  // Check if the line contains a scoped variable pattern followed by a global object
  SCOPED_VAR_PATTERN.lastIndex = 0;
  const match = SCOPED_VAR_PATTERN.exec(line);
  if (!match) {
    return false;
  }

  // Check what comes after the scoped variable pattern
  const afterPattern = line.substring(match.index + match[0].length);

  // Check if any global object appears immediately after the pattern
  for (const globalObj of GLOBAL_OBJECTS) {
    // Match global object as a word boundary (to avoid matching parts of longer words)
    const globalPattern = new RegExp(
      `\\b${globalObj.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\b`,
    );
    if (globalPattern.test(afterPattern)) {
      return true;
    }
  }

  return false;
}

/**
 * Checks if a line contains a direct reference to a global object (not wrapped in scoped variable)
 */
export function hasDirectGlobalObject(line: string): boolean {
  for (const globalObj of GLOBAL_OBJECTS) {
    const pattern = new RegExp(
      `\\b${globalObj.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\b`,
    );
    if (pattern.test(line) && !hasGlobalObjectTransformation(line)) {
      return true;
    }
  }
  return false;
}

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
 * Returns false if one line has a global object and the other has it wrapped in a scoped variable
 */
export function linesDifferOnlyByUnminifiedRefs(
  line1: string,
  line2: string,
): boolean {
  // Critical check: if one line has a direct global object reference and the other
  // has it wrapped in a scoped variable, this is a breaking change
  const line1HasGlobal = hasDirectGlobalObject(line1);
  const line2HasGlobal = hasDirectGlobalObject(line2);
  const line1HasScopedGlobal = hasGlobalObjectTransformation(line1);
  const line2HasScopedGlobal = hasGlobalObjectTransformation(line2);

  // If one line has a direct global and the other has it scoped, it's a breaking change
  if (
    (line1HasGlobal && line2HasScopedGlobal) ||
    (line2HasGlobal && line1HasScopedGlobal)
  ) {
    return false;
  }

  return normalizeUnminifiedRefs(line1) === normalizeUnminifiedRefs(line2);
}

/**
 * Normalizes source map URLs in a line by replacing them with a placeholder
 */
export function normalizeSourceMapUrl(line: string): string {
  // Reset regex lastIndex to avoid state issues
  SOURCE_MAP_URL_PATTERN.lastIndex = 0;
  // Use replace directly - it will return the original string if no match
  // Preserve trailing newlines if they exist
  const match = line.match(SOURCE_MAP_URL_PATTERN);
  if (match) {
    const trailingNewline = line.endsWith('\n') ? '\n' : '';
    return line.replace(
      SOURCE_MAP_URL_PATTERN,
      SOURCE_MAP_URL_PLACEHOLDER + trailingNewline,
    );
  }
  return line;
}

/**
 * Checks if two lines differ only by source map URLs
 */
export function linesDifferOnlyBySourceMapUrl(
  line1: string,
  line2: string,
): boolean {
  return normalizeSourceMapUrl(line1) === normalizeSourceMapUrl(line2);
}

/**
 * Checks if a character at a given position is part of a valid JavaScript identifier context
 * This ensures we don't match parts of longer words or strings
 */
function isValidIdentifierContext(
  line: string,
  start: number,
  end: number,
): boolean {
  // Check character before the match
  const before = start > 0 ? line[start - 1] : ' ';
  // Check character after the match
  const after = end < line.length ? line[end] : ' ';

  // Valid identifier contexts:
  // - Start of line or after non-word character (space, operator, etc.)
  // - Before non-word character (space, operator, etc.)
  // - After/before common JS operators: =, +, -, *, /, %, <, >, &, |, !, ?, :, ;, ,, ., (, ), [, ], {, }
  const validBefore = /[\s=+\-*/%<>&|!?:;,.()[\]{}]/.test(before);
  const validAfter = /[\s=+\-*/%<>&|!?:;,.()[\]{}]/.test(after);

  return validBefore && validAfter;
}

/**
 * Finds all single-character or short variable names in a line
 * Returns a set of variable names that appear as identifiers (not in strings or parts of words)
 */
function findShortVariables(line: string): Set<string> {
  // Match short identifiers (1-3 characters) that are likely minified variables
  // Use word boundaries and look for patterns like: (var=, var., var), var, etc.
  const shortVarPattern = /\b([a-zA-Z_$][a-zA-Z0-9_$]{0,2})\b/g;
  const variables = new Set<string>();
  let match;

  shortVarPattern.lastIndex = 0;
  while ((match = shortVarPattern.exec(line)) !== null) {
    const varName = match[1];
    const start = match.index;
    const end = start + varName.length;

    // Skip common keywords and long names
    if (
      varName.length <= 3 &&
      ![
        'var',
        'let',
        'const',
        'for',
        'if',
        'new',
        'return',
        'this',
        'null',
        'true',
        'false',
        'undefined',
      ].includes(varName) &&
      isValidIdentifierContext(line, start, end)
    ) {
      variables.add(varName);
    }
  }

  return variables;
}

/**
 * Tries to find a variable swap mapping that makes two lines identical
 * Returns the mapping if found, null otherwise
 */
function findSwapMapping(
  line1: string,
  line2: string,
): Map<string, string> | null {
  const vars1 = findShortVariables(line1);
  const vars2 = findShortVariables(line2);

  // Find variables that appear in line1 but not line2, and vice versa
  const onlyIn1 = Array.from(vars1).filter((v) => !vars2.has(v));
  const onlyIn2 = Array.from(vars2).filter((v) => !vars1.has(v));

  // If no differences, lines are identical
  if (onlyIn1.length === 0 && onlyIn2.length === 0) {
    return null;
  }

  // If different number of differing variables, can't be a simple swap
  if (onlyIn1.length !== onlyIn2.length || onlyIn1.length === 0) {
    return null;
  }

  // Try all possible pairings (for small sets, this is feasible)
  // For now, try 1:1 mapping (most common case)
  if (onlyIn1.length === 1 && onlyIn2.length === 1) {
    const mapping = new Map<string, string>();
    mapping.set(onlyIn1[0], onlyIn2[0]);

    // Test if this mapping works with word-boundary validation
    const escapedVar = onlyIn1[0].replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const regex = new RegExp(`\\b${escapedVar}\\b`, 'g');

    // First, validate that all matches are in valid identifier contexts
    let match;
    regex.lastIndex = 0;
    const matches: Array<{start: number; end: number}> = [];
    while ((match = regex.exec(line1)) !== null) {
      const start = match.index;
      const end = start + match[0].length;
      if (!isValidIdentifierContext(line1, start, end)) {
        // Invalid context found - this swap is not valid
        return null;
      }
      matches.push({start, end});
    }

    // If all matches are valid, perform the replacement
    let testLine = line1;
    // Replace from end to start to preserve offsets
    for (let i = matches.length - 1; i >= 0; i--) {
      const {start, end} = matches[i];
      testLine =
        testLine.substring(0, start) + onlyIn2[0] + testLine.substring(end);
    }

    if (testLine === line2) {
      return mapping;
    }
  }

  // For multiple swaps, try matching by occurrence count
  // Count occurrences of each variable
  const count1 = new Map<string, number>();
  const count2 = new Map<string, number>();

  for (const v of onlyIn1) {
    const regex = new RegExp(
      `\\b${v.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\b`,
      'g',
    );
    count1.set(v, (line1.match(regex) || []).length);
  }

  for (const v of onlyIn2) {
    const regex = new RegExp(
      `\\b${v.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\b`,
      'g',
    );
    count2.set(v, (line2.match(regex) || []).length);
  }

  // Try to match variables with same occurrence counts
  const mapping = new Map<string, string>();
  const used2 = new Set<string>();

  const sorted1 = Array.from(onlyIn1).sort(
    (a, b) => (count1.get(b) || 0) - (count1.get(a) || 0),
  );

  for (const v1 of sorted1) {
    const count = count1.get(v1)!;
    let found = false;

    for (const v2 of onlyIn2) {
      if (used2.has(v2)) continue;
      if (count2.get(v2) === count) {
        mapping.set(v1, v2);
        used2.add(v2);
        found = true;
        break;
      }
    }

    if (!found) {
      return null;
    }
  }

  // Test if this mapping works with word-boundary validation
  // Validate all matches before doing any replacements
  const sortedMappings = Array.from(mapping.entries()).sort(
    (a, b) => b[0].length - a[0].length,
  );

  // First pass: validate all variable occurrences are in valid contexts
  for (const [from] of sortedMappings) {
    const escapedVar = from.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const regex = new RegExp(`\\b${escapedVar}\\b`, 'g');
    let match;
    regex.lastIndex = 0;
    while ((match = regex.exec(line1)) !== null) {
      const start = match.index;
      const end = start + match[0].length;
      if (!isValidIdentifierContext(line1, start, end)) {
        // Invalid context found - this swap is not valid
        return null;
      }
    }
  }

  // Second pass: perform replacements (from longest to shortest to preserve offsets)
  let testLine = line1;
  for (const [from, to] of sortedMappings) {
    const escapedVar = from.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const regex = new RegExp(`\\b${escapedVar}\\b`, 'g');

    // Collect all matches and replace from end to start
    const matches: Array<{start: number; end: number}> = [];
    let match;
    regex.lastIndex = 0;
    while ((match = regex.exec(testLine)) !== null) {
      matches.push({start: match.index, end: match.index + match[0].length});
    }

    // Replace from end to start to preserve offsets
    for (let i = matches.length - 1; i >= 0; i--) {
      const {start, end} = matches[i];
      testLine = testLine.substring(0, start) + to + testLine.substring(end);
    }
  }

  if (testLine === line2) {
    return mapping;
  }

  return null;
}

/**
 * Checks if two lines differ only by swapped variables
 * Returns false if one line has a global object and the other has it wrapped in a scoped variable
 */
export function linesDifferOnlyBySwappedVariables(
  line1: string,
  line2: string,
): boolean {
  // Critical check: if one line has a direct global object reference and the other
  // has it wrapped in a scoped variable, this is a breaking change
  const line1HasGlobal = hasDirectGlobalObject(line1);
  const line2HasGlobal = hasDirectGlobalObject(line2);
  const line1HasScopedGlobal = hasGlobalObjectTransformation(line1);
  const line2HasScopedGlobal = hasGlobalObjectTransformation(line2);

  // If one line has a direct global and the other has it scoped, it's a breaking change
  if (
    (line1HasGlobal && line2HasScopedGlobal) ||
    (line2HasGlobal && line1HasScopedGlobal)
  ) {
    return false;
  }

  const mapping = findSwapMapping(line1, line2);
  return mapping !== null && mapping.size > 0;
}
