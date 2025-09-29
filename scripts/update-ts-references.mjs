#!/usr/bin/env node
/* eslint-disable no-console */

import fs from 'node:fs';
import url from 'node:url';
import path from 'node:path';
import glob from 'glob';
import pkg from 'json5';
import { parse as astParse } from '@ast-grep/napi';

const {parse} = pkg;

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const __root = path.dirname(__dirname); // Go up one level to project root

const IGNORED_PATTERNS = [
  'apvm',
  'node_modules',
  'node-resolver-core/test/fixture',
  'test/fixtures',
  'examples',
  'example',
  'integration-tests',
  'workers/test/integration',
  'fixtures',
  'fixture',
  'template',
  'lib',
  'packages/dev/atlaspack-inspector',
];

/**
 * Check if a package's source code imports its package.json file using AST analysis
 */
function checkForPackageJsonImports(packagePath) {
  const srcPath = path.join(packagePath, 'src');

  // Check if src directory exists
  if (!fs.existsSync(srcPath)) {
    return false;
  }

  try {
    // Get all TypeScript/JavaScript files in src directory recursively
    const sourceFiles = glob.sync('**/*.{ts,tsx,js,jsx}', {
      cwd: srcPath,
      absolute: true
    });

    for (const filePath of sourceFiles) {
      try {
        const content = fs.readFileSync(filePath, 'utf8');
        if (!content.includes('package.json')) continue;

        // Determine language for ast-grep based on file extension
        const ext = path.extname(filePath);
        const language = ext === '.ts' || ext === '.tsx' ? 'typescript' : 'javascript';

        // Parse the file into an AST
        const ast = astParse(language, content);
        const root = ast.root();

        // Define patterns to match different types of package.json imports
        const patterns = [
          // ES6 import statements - default imports
          'import $IDENTIFIER from $STRING',
          // ES6 import statements - named imports
          'import { $$$NAMES } from $STRING',
          // ES6 import statements - namespace imports
          'import * as $IDENTIFIER from $STRING',
          // CommonJS require statements
          'require($STRING)',
        ];

        // Check each pattern
        for (const pattern of patterns) {
          const matches = root.findAll(pattern);
          for (const match of matches) {
            try {
              // Get the import/require path
              const stringNodes = match.findAll('$STRING');
              for (const stringNode of stringNodes) {
                const importPath = stringNode.text();
                // Remove quotes and check if it's a relative import of package.json
                const cleanPath = importPath.replace(/['"]/g, '');
                // Only match relative imports to own package.json (starts with ./ or ../)
                if ((cleanPath.startsWith('./') || cleanPath.startsWith('../')) &&
                    cleanPath.endsWith('/package.json')) {
                  return true;
                }
              }
            } catch {
              // Continue checking other matches if this one fails
              continue;
            }
          }
        }
      } catch {
        // Skip files that can't be parsed, but don't fail the whole process
        continue;
      }
    }
  } catch {
    // If we can't scan the directory, assume no imports
    return false;
  }

  return false;
}

/**
 * Get all package information from the monorepo
 */
function getAllPackages(frozen = false) {
  const packages = new Map();
  let validationErrors = 0;
  let validationFixes = 0;

  for (const packageJsonPathRel of glob.sync('packages/**/*/package.json', {
    cwd: __root,
  })) {
    if (
      IGNORED_PATTERNS.some((pattern) => packageJsonPathRel.includes(pattern))
    ) {
      continue;
    }

    const packageJsonPath = path.join(__root, packageJsonPathRel);
    const packagePath = path.dirname(packageJsonPath);
    const tsconfigPath = path.join(packagePath, 'tsconfig.json');

    // Only include packages that have a tsconfig.json
    if (!fs.existsSync(tsconfigPath)) {
      continue;
    }

    try {
      let pkg, tsconfig;
      try {
        pkg = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
      } catch (error) {
        throw new Error(
          `Error loading or parsing package.json: ${error.message}`,
        );
      }
      try {
        tsconfig = parse(fs.readFileSync(tsconfigPath, 'utf8'));
      } catch (error) {
        throw new Error(
          `Error loading or parsing tsconfig.json: ${error.message}`,
        );
      }

      // Validate tsconfig assertions
      const relativeTsconfigPath = path.relative(__root, tsconfigPath);

      // Calculate expected extends path relative to package directory
      const expectedExtends = path.relative(packagePath, path.join(__root, 'tsconfig.base.json'));

      let tsconfigChanged = false;

      // Assertion 1: Should extend tsconfig.base.json from the root
      if (!tsconfig.extends || tsconfig.extends !== expectedExtends) {
        if (frozen) {
          console.error(
            `❌ ${relativeTsconfigPath}: Expected "extends": "${expectedExtends}", but got: "${tsconfig.extends || 'undefined'}"`,
          );
          validationErrors++;
        } else {
          console.log(
            `🔧 ${relativeTsconfigPath}: Fixing "extends" path from "${tsconfig.extends || 'undefined'}" to "${expectedExtends}"`,
          );
          tsconfig.extends = expectedExtends;
          tsconfigChanged = true;
          validationFixes++;
        }
      }

      // Assertion 2: Should include compilerOptions.composite: true
      if (!tsconfig.compilerOptions?.composite) {
        console.error(
          `❌ ${relativeTsconfigPath}: Expected "compilerOptions.composite": true, but got: ${tsconfig.compilerOptions?.composite || 'undefined'}`,
        );
        validationErrors++;
        continue;
      }

      // Assertion 3: Check if source code imports package.json, and if so, validate it's in include array
      const hasPackageJsonImport = checkForPackageJsonImports(packagePath);
      if (hasPackageJsonImport && (!tsconfig.include || !tsconfig.include.includes('./package.json'))) {
        if (frozen) {
          console.error(
            `❌ ${relativeTsconfigPath}: Source code imports package.json but "./package.json" is missing from "include" array. Got: ${JSON.stringify(tsconfig.include || [])}`,
          );
          validationErrors++;
        } else {
          console.log(
            `🔧 ${relativeTsconfigPath}: Adding "./package.json" to include array (source code imports package.json)`,
          );
          if (!tsconfig.include) {
            tsconfig.include = [];
          }
          tsconfig.include.push('./package.json');
          tsconfigChanged = true;
          validationFixes++;
        }
      }

      // Write the updated tsconfig if changes were made
      if (tsconfigChanged) {
        fs.writeFileSync(
          tsconfigPath,
          JSON.stringify(tsconfig, null, 2) + '\n',
        );
      }

      packages.set(pkg.name, {
        name: pkg.name,
        path: packagePath,
        relativePath: path.relative(__root, packagePath),
        tsconfigPath: relativeTsconfigPath,
        dependencies: {
          ...pkg.dependencies,
          ...pkg.devDependencies,
        },
        tsconfig,
      });
    } catch (error) {
      console.warn(
        `Error reading package at ${path.dirname(packageJsonPathRel)}:`,
        error.message,
      );
    }
  }

  return { packages, validationErrors, validationFixes };
}

/**
 * Build dependency graph and update tsconfig references
 */
function updateTsConfigReferences(packages, frozen) {
  let totalUpdates = 0;

  for (const [packageName, packageInfo] of packages) {
    const currentTsconfig = packageInfo.tsconfig;
    const requiredReferences = new Set();

    // Find all @atlaspack dependencies that have corresponding packages
    for (const depName of Object.keys(packageInfo.dependencies || {})) {
      if (depName.startsWith('@atlaspack/') && packages.has(depName)) {
        const depPackage = packages.get(depName);
        const relativePath = path.relative(
          packageInfo.path,
          path.join(__root, depPackage.tsconfigPath),
        );
        requiredReferences.add(relativePath);
      }
    }

    // Preserve existing order while adding/removing references
    const currentReferences = currentTsconfig.references || [];
    const newReferences = [];

    // First, keep existing references that are still required (preserving order)
    for (const ref of currentReferences) {
      if (requiredReferences.has(ref.path)) {
        newReferences.push({path: ref.path});
        requiredReferences.delete(ref.path);
      }
    }

    // Then add any new references that weren't in the original list (sorted for consistency)
    const newRefs = Array.from(requiredReferences)
      .sort()
      .map((path) => ({path}));
    newReferences.push(...newRefs);

    // Check if references have changed
    const referencesChanged =
      currentReferences.length !== newReferences.length ||
      !currentReferences.every(
        (ref, index) => ref.path === newReferences[index]?.path,
      );

    if (referencesChanged) {
      currentTsconfig.references = newReferences;

      if (frozen) {
        console.log(`Skipping ${packageName}:`);
      } else {
        const tsconfigPath = path.join(packageInfo.path, 'tsconfig.json');
        fs.writeFileSync(
          tsconfigPath,
          JSON.stringify(currentTsconfig, null, 2) + '\n',
        );

        console.log(`Updated ${packageName}:`);
      }

      console.log(
        `  References: ${newReferences.map((r) => r.path).join(', ') || 'none'}`,
      );

      totalUpdates += 1;
    }
  }

  return totalUpdates;
}

/**
 * Update the root tsconfig.paths.json with all composite projects
 */
function updateRootReferences(packages, frozen) {
  const rootTsconfigPath = path.join(__root, 'tsconfig.paths.json');

  let rootTsconfig = {};
  if (fs.existsSync(rootTsconfigPath)) {
    rootTsconfig = parse(fs.readFileSync(rootTsconfigPath, 'utf8'));
  }

  // Get all required references
  const requiredReferences = new Set(
    Array.from(packages.values()).map((pkg) => `./${pkg.tsconfigPath}`),
  );

  // Preserve existing order while adding/removing references
  const currentReferences = rootTsconfig.references || [];
  const allReferences = [];

  // First, keep existing references that are still required (preserving order)
  for (const ref of currentReferences) {
    if (requiredReferences.has(ref.path)) {
      allReferences.push({path: ref.path});
      requiredReferences.delete(ref.path);
    }
  }

  // Then add any new references that weren't in the original list (sorted for consistency)
  const newRefs = Array.from(requiredReferences)
    .sort()
    .map((path) => ({path}));
  allReferences.push(...newRefs);

  const referencesChanged =
    currentReferences.length !== allReferences.length ||
    !currentReferences.every(
      (ref, index) => ref.path === allReferences[index]?.path,
    );

  if (referencesChanged) {
    rootTsconfig.references = allReferences;

    if (frozen) {
      console.log(
        `\nSkipping updating tsconfig.paths.json with ${allReferences.length} references`,
      );
    } else {
      fs.writeFileSync(
        rootTsconfigPath,
        JSON.stringify(rootTsconfig, null, 2) + '\n',
      );
      console.log(
        `\nUpdated tsconfig.paths.json with ${allReferences.length} references`,
      );
    }

    return true;
  }

  return false;
}

function main() {
  const frozen = process.argv.includes('--frozen');

  console.log('🔍 Scanning for TypeScript packages...');

  const { packages, validationErrors, validationFixes } = getAllPackages(frozen);
  console.log(
    `Found ${packages.size} TypeScript packages with composite: true`,
  );

  if (validationFixes > 0) {
    console.log(
      `🔧 Fixed ${validationFixes} validation issue${validationFixes === 1 ? '' : 's'}`,
    );
  }

  console.log('\n📝 Updating individual package references...');
  const packageUpdates = updateTsConfigReferences(packages, frozen);

  console.log('\n📝 Updating references in tsconfig.paths.json...');
  const rootUpdated = updateRootReferences(packages, frozen);

  console.log('\n✅ Done!');
  console.log(
    `  - Updated ${packageUpdates} individual package tsconfig.json files`,
  );
  console.log(
    `  - tsconfig.paths.json ${rootUpdated ? 'updated' : 'unchanged'}`,
  );
  if (validationFixes > 0) {
    console.log(
      `  - Fixed ${validationFixes} validation issue${validationFixes === 1 ? '' : 's'}`,
    );
  }

  if (frozen && (packageUpdates > 0 || rootUpdated || validationFixes > 0)) {
    console.log(
      '\n❌ Exiting with error. Rerun without --frozen to update references and fix validation issues.',
    );
    process.exitCode = 1;
  }

  if (validationErrors > 0) {
    console.log(
      `\n❌ Found ${validationErrors} validation error${validationErrors === 1 ? '' : 's'}. Please fix the issues above.`,
    );
    process.exitCode = 1;
  }
}

main();
