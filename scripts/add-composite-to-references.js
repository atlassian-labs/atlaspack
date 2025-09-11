#!/usr/bin/env node

/* eslint-disable no-console */
import * as fs from 'node:fs';
import * as url from 'node:url';
import * as path from 'node:path';
import glob from 'glob';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const __root = __dirname;

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
function updateTsConfigReferences(packages) {
  let totalUpdates = 0;

  for (const [packageName, packageInfo] of packages) {
    const currentTsconfig = packageInfo.tsconfig;
    const newReferences = [];

    // Find all @atlaspack dependencies that have corresponding packages
    for (const depName of Object.keys(packageInfo.dependencies || {})) {
      if (depName.startsWith('@atlaspack/') && packages.has(depName)) {
        const depPackage = packages.get(depName);
        const relativePath = path.relative(
          packageInfo.path,
          path.join(__root, depPackage.tsconfigPath),
        );
        newReferences.push({path: relativePath});
      }
    }

    // Sort references by path for consistency
    newReferences.sort((a, b) => a.path.localeCompare(b.path));

    // Check if references have changed
    const currentReferences = currentTsconfig.references || [];
    const referencesChanged =
      currentReferences.length !== newReferences.length ||
      !currentReferences.every(
        (ref, index) => ref.path === newReferences[index]?.path,
      );

    if (referencesChanged) {
      currentTsconfig.references = newReferences;

      const tsconfigPath = path.join(packageInfo.path, 'tsconfig.json');
      fs.writeFileSync(
        tsconfigPath,
        JSON.stringify(currentTsconfig, null, 2) + '\n',
      );

      console.log(`Updated ${packageName}:`);
      console.log(
        `  References: ${newReferences.map((r) => r.path).join(', ') || 'none'}`,
      );
      totalUpdates++;
    }
  }

  return totalUpdates;
}

/**
 * Update the root tsconfig.paths.json with all composite projects
 */
function updateRootReferences(packages) {
  const rootTsconfigPath = path.join(__root, 'tsconfig.paths.json');

  let rootTsconfig = {};
  if (fs.existsSync(rootTsconfigPath)) {
    rootTsconfig = JSON.parse(fs.readFileSync(rootTsconfigPath, 'utf8'));
  }

  const allReferences = Array.from(packages.values())
    .map((pkg) => ({path: `./${pkg.tsconfigPath}`}))
    .sort((a, b) => a.path.localeCompare(b.path));

  const currentReferences = rootTsconfig.references || [];
  const referencesChanged =
    currentReferences.length !== allReferences.length ||
    !currentReferences.every(
      (ref, index) => ref.path === allReferences[index]?.path,
    );

  if (referencesChanged) {
    rootTsconfig.references = allReferences;
    fs.writeFileSync(
      rootTsconfigPath,
      JSON.stringify(rootTsconfig, null, 2) + '\n',
    );
    console.log(
      `\nUpdated root tsconfig.paths.json with ${allReferences.length} references`,
    );
    return true;
  }

  return false;
}

/**
 * Main function
 */
function main() {
  console.log('🔍 Scanning for TypeScript packages...');

  const packages = getAllPackages();
  console.log(
    `Found ${packages.size} TypeScript packages with composite: true`,
  );

  console.log('\n📝 Updating individual package references...');
  const packageUpdates = updateTsConfigReferences(packages);

  console.log('\n📝 Updating root references...');
  const rootUpdated = updateRootReferences(packages);

  console.log('\n✅ Done!');
  console.log(
    `  - Updated ${packageUpdates} individual package tsconfig.json files`,
  );
  console.log(
    `  - Root tsconfig.paths.json ${rootUpdated ? 'updated' : 'unchanged'}`,
  );

  if (packageUpdates > 0 || rootUpdated) {
    console.log(
      '\n💡 You may want to run `yarn check-ts` to verify the changes',
    );
  }
}

// Run the script
main();
