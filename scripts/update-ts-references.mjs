#!/usr/bin/env node
/* eslint-disable no-console */

import fs from 'node:fs';
import url from 'node:url';
import path from 'node:path';
import glob from 'glob';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const __root = path.dirname(__dirname); // Go up one level to project root

const IGNORED_PATTERNS = [
  'apvm',
  'node_modules',
  'node-resolver-core/test/fixture',
  'test/fixtures',
  'examples',
  'integration-tests',
  'workers/test/integration',
  'fixtures',
  'fixture',
  'template',
  'lib',
];

/**
 * Get all package information from the monorepo
 */
function getAllPackages() {
  const packages = new Map();

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
      const pkg = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
      const tsconfig = JSON.parse(fs.readFileSync(tsconfigPath, 'utf8'));

      // Only include packages with composite: true
      if (!tsconfig.compilerOptions?.composite) {
        continue;
      }

      const relativeTsconfigPath = path.relative(__root, tsconfigPath);

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
        `Error reading package at ${packageJsonPathRel}:`,
        error.message,
      );
    }
  }

  return packages;
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
    rootTsconfig = JSON.parse(fs.readFileSync(rootTsconfigPath, 'utf8'));
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

  const packages = getAllPackages();
  console.log(
    `Found ${packages.size} TypeScript packages with composite: true`,
  );

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

  if (frozen && (packageUpdates > 0 || rootUpdated)) {
    console.log(
      '\n❌ Exiting with error. Rerun without --frozen to update references.',
    );
    process.exitCode = 1;
  }
}

main();
