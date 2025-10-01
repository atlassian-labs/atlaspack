#!/usr/bin/env node
/* eslint-disable no-console */
/**
 * Find unused feature flags in the Atlaspack codebase
 *
 * This script:
 * 1. Reads all feature flag names from DEFAULT_FEATURE_FLAGS in packages/core/feature-flags/src/index.ts
 * 2. Uses fast-glob to find all TypeScript/JavaScript/Rust files in packages/ and crates/ (excluding lib/ directories)
 * 3. Lazily filters files that contain feature flag patterns for efficiency
 * 4. Uses AST parsing via @ast-grep/napi to find:
 *    - TypeScript/JavaScript: getFeatureFlag() and getFeatureFlagValue() function calls
 *    - TypeScript/JavaScript: *.featureFlags.<name> property access patterns
 *    - TypeScript/JavaScript: *.featureFlags?.<name> optional chaining patterns
 *    - Rust: *.feature_flags.bool_enabled("<name>") method calls
 * 5. Returns a list of feature flags that are defined but never used (ignoring example flags)
 *
 * Dependencies: fast-glob, @ast-grep/napi
 * Usage: node scripts/find-unused-feature-flags.js
 */

const fs = require('fs');
const path = require('path');
const fg = require('fast-glob');
const {parse, Lang} = require('@ast-grep/napi');

const REPO_ROOT = path.resolve(__dirname, '..');
const FEATURE_FLAGS_FILE = path.join(
  REPO_ROOT,
  'packages/core/feature-flags/src/index.ts',
);
const PACKAGES_DIR = path.join(REPO_ROOT, 'packages');
const CRATES_DIR = path.join(REPO_ROOT, 'crates');

/**
 * Extract feature flag names from the DEFAULT_FEATURE_FLAGS object using AST parsing
 */
function extractFeatureFlagNames() {
  try {
    const content = fs.readFileSync(FEATURE_FLAGS_FILE, 'utf8');

    // Parse the TypeScript file using @ast-grep/napi
    const ast = parse(Lang.TypeScript, content);
    const root = ast.root();

    const flagNames = [];

    // Find all pairs (property assignments) inside the DEFAULT_FEATURE_FLAGS variable declarator
    const properties = root.findAll({
      rule: {
        kind: 'pair',
        inside: {
          kind: 'variable_declarator',
          stopBy: 'end',
          has: {
            field: 'name',
            regex: 'DEFAULT_FEATURE_FLAGS',
          },
        },
      },
    });

    for (const property of properties) {
      // Get the key field which contains the property name
      const keyNode = property.field('key');
      if (keyNode && keyNode.text()) {
        let flagName = keyNode.text().trim();

        // Remove quotes if it's a string literal property name
        if (
          (flagName.startsWith('"') && flagName.endsWith('"')) ||
          (flagName.startsWith("'") && flagName.endsWith("'")) ||
          (flagName.startsWith('`') && flagName.endsWith('`'))
        ) {
          flagName = flagName.slice(1, -1);
        }

        // Only add valid identifier names
        if (flagName && /^[a-zA-Z_$][a-zA-Z0-9_$]*$/.test(flagName)) {
          flagNames.push(flagName);
        }
      }
    }

    if (flagNames.length === 0) {
      throw new Error(
        'Could not find any feature flag properties in DEFAULT_FEATURE_FLAGS',
      );
    }

    return flagNames;
  } catch (error) {
    console.error('Error extracting feature flag names:', error.message);
    process.exit(1);
  }
}

/**
 * Search for feature flag usage in packages/ and crates/ directories using fast-glob and AST parsing
 */
function findGetFeatureFlagUsage() {
  const usedFlags = new Set();

  try {
    // Use fast-glob to find all TypeScript/JavaScript files in packages and Rust files in crates
    const tsJsPattern = `${PACKAGES_DIR}/**/src/**/*.{ts,tsx,js,jsx}`;
    const rustPattern = `${CRATES_DIR}/**/*.rs`;
    const ignorePatterns = ['**/lib/**', '**/node_modules/**', '**/target/**'];

    console.log('   📁 Finding TypeScript/JavaScript and Rust files...');

    const tsJsFiles = fg.sync(tsJsPattern, {
      ignore: ignorePatterns,
      absolute: true,
      onlyFiles: true,
    });

    const rustFiles = fg.sync(rustPattern, {
      ignore: ignorePatterns,
      absolute: true,
      onlyFiles: true,
    });

    const allFiles = [...tsJsFiles, ...rustFiles];

    console.log(
      `   📄 Found ${allFiles.length} source files (${tsJsFiles.length} TS/JS, ${rustFiles.length} Rust), filtering for feature flag usage...`,
    );

    // Filter files that contain feature flag patterns and parse them
    let candidateFiles = [];
    for (const filePath of allFiles) {
      try {
        const content = fs.readFileSync(filePath, 'utf8');
        // Check for TypeScript/JavaScript patterns or Rust patterns
        if (
          content.includes('getFeatureFlag') ||
          content.includes('getFeatureFlagValue') ||
          content.includes('featureFlags') ||
          content.includes('feature_flags')
        ) {
          candidateFiles.push(filePath);
        }
      } catch (error) {
        // Skip files that can't be read
        console.warn(`   ⚠️  Could not read file: ${filePath}: ${error}`);
      }
    }

    console.log(
      `   🔍 Found ${candidateFiles.length} files containing feature flag patterns`,
    );

    // Now parse each candidate file using AST
    for (const filePath of candidateFiles) {
      const fileFlags = parseFileForFlags(filePath);
      fileFlags.forEach((flag) => usedFlags.add(flag));
    }

    return Array.from(usedFlags);
  } catch (error) {
    console.error('Error searching for getFeatureFlag usage:', error.message);
    return [];
  }
}

/**
 * Parse a file using ast-grep to find feature flag usage patterns in TypeScript/JavaScript/Rust
 */
function parseFileForFlags(filePath) {
  const usedFlags = new Set();

  try {
    const content = fs.readFileSync(filePath, 'utf8');

    // Quick string check first - only parse if feature flag patterns are mentioned
    if (
      !content.includes('getFeatureFlag') &&
      !content.includes('getFeatureFlagValue') &&
      !content.includes('featureFlags') &&
      !content.includes('feature_flags')
    ) {
      return Array.from(usedFlags);
    }

    // Skip the feature flags definition file itself
    if (filePath.includes('feature-flags/src/index.ts')) {
      return Array.from(usedFlags);
    }

    // Determine language based on file extension
    let language;

    if (filePath.endsWith('.rs')) {
      language = Lang.Rust;
    } else if (/\.tsx?$/.test(filePath)) {
      language = Lang.TypeScript;
    } else {
      language = Lang.JavaScript;
    }

    // @ast-grep/napi currently fails to parse Rust files properly, so this is a bit of a hack
    if (language === Lang.Rust) {
      // Handle Rust patterns: *.feature_flags.bool_enabled("<flag>")
      const rustPattern = /bool_enabled\("([^"]+)"\)/g;
      let match;
      while ((match = rustPattern.exec(content)) !== null) {
        const flagName = match[1];
        if (flagName) {
          usedFlags.add(flagName);
        }
      }
    } else {
      // Handle TypeScript/JavaScript patterns
      // Parse the file
      const ast = parse(language, content);
      const root = ast.root();

      // Find all getFeatureFlag calls with string arguments
      const flagAccessCalls = root.findAll({
        rule: {
          any: [
            {pattern: 'getFeatureFlag($FLAG)'},
            {pattern: 'getFeatureFlagValue($FLAG)'},
            {pattern: '$OBJ.featureFlags.$FLAG'},
            {pattern: '$OBJ.featureFlags?.$FLAG'},
          ],
        },
      });

      for (const call of flagAccessCalls) {
        const flagMatch = call.getMatch('FLAG');
        if (flagMatch && flagMatch.text()) {
          let flagName = flagMatch.text().trim();
          // Check if it's a string literal
          if (
            (flagName.startsWith('"') && flagName.endsWith('"')) ||
            (flagName.startsWith("'") && flagName.endsWith("'")) ||
            (flagName.startsWith('`') && flagName.endsWith('`'))
          ) {
            // Remove quotes from string literal
            flagName = flagName.slice(1, -1);
          }
          if (flagName) {
            usedFlags.add(flagName);
          }
        }
      }
    }
  } catch (error) {
    console.warn(`Error parsing file ${filePath}:`, error.message);
  }

  return Array.from(usedFlags);
}

/**
 * Main function to find unused feature flags
 */
function main() {
  console.log('🏁 Finding unused feature flags...\n');

  console.log('📖 Extracting feature flag names from DEFAULT_FEATURE_FLAGS...');
  const allFlags = extractFeatureFlagNames();
  console.log(`   Found ${allFlags.length} feature flags`);

  console.log(
    '\n🔍 Searching for feature flag usage patterns in packages/ and crates/...',
  );
  const usedFlags = findGetFeatureFlagUsage();
  console.log(
    `   Found ${usedFlags.length} flags referenced across all usage patterns`,
  );

  console.log('\n📋 Used flags:');
  if (usedFlags.length > 0) {
    usedFlags.sort().forEach((flag) => console.log(`   ✅ ${flag}`));
  } else {
    console.log('   None found');
  }

  console.log('\n🚫 Unused flags:');
  const unusedFlags = allFlags.filter(
    (flag) => !usedFlags.includes(flag) && !flag.startsWith('example'),
  );

  if (unusedFlags.length > 0) {
    unusedFlags.sort().forEach((flag) => console.log(`   ❌ ${flag}`));
    console.log(
      `\n📊 Summary: ${unusedFlags.length} of ${allFlags.length} feature flags are unused (${Math.round((unusedFlags.length / allFlags.length) * 100)}%) (excluding example flags)`,
    );
  } else {
    console.log(
      '   None! All feature flags are being used (excluding example flags).',
    );
    console.log(`\n📊 Summary: All non-example feature flags are in use.`);
  }

  // Return the unused flags for programmatic use
  return unusedFlags;
}

// Run the script if called directly
if (require.main === module) {
  try {
    const unusedFlags = main();
    if (unusedFlags.length > 0) {
      process.exitCode = 1;
    }
  } catch (error) {
    console.error('Error running script:', error);
    process.exitCode = 1;
  }
}

module.exports = {main, extractFeatureFlagNames, findGetFeatureFlagUsage};
