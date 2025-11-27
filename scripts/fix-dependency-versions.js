#!/usr/bin/env node
/* eslint-disable no-console */

const fs = require('fs');
const path = require('path');
const glob = require('glob');

/**
 * Script to fix dependency versions across all package.json files in the monorepo.
 * For every workspace package, updates dependency references to use the latest version.
 */

// Find all package.json files in the repo
function findAllPackageJsonFiles() {
  const rootDir = path.resolve(__dirname, '..');
  const pattern = '**/package.json';

  const files = glob.sync(pattern, {
    cwd: rootDir,
    ignore: ['node_modules/**', 'target/**', 'tmp/**', '**/node_modules/**'],
  });

  return files.map((file) => path.resolve(rootDir, file));
}

// Find all workspace packages based on lerna configuration
function findWorkspacePackages() {
  const rootDir = path.resolve(__dirname, '..');

  // Read lerna.json to get workspace patterns
  const lernaPath = path.join(rootDir, 'lerna.json');
  const lernaConfig = JSON.parse(fs.readFileSync(lernaPath, 'utf8'));

  // Also check root package.json for workspaces
  const rootPackagePath = path.join(rootDir, 'package.json');
  const rootPackage = JSON.parse(fs.readFileSync(rootPackagePath, 'utf8'));

  const workspacePatterns = [
    ...(lernaConfig.packages || []),
    ...(rootPackage.workspaces || []),
  ];

  console.log('Workspace patterns:', workspacePatterns);

  const workspacePackages = new Map();

  for (const pattern of workspacePatterns) {
    const packageDirs = glob.sync(pattern, {cwd: rootDir});

    for (const dir of packageDirs) {
      const packageJsonPath = path.join(rootDir, dir, 'package.json');

      if (fs.existsSync(packageJsonPath)) {
        try {
          const packageJson = JSON.parse(
            fs.readFileSync(packageJsonPath, 'utf8'),
          );

          if (packageJson.name && packageJson.version) {
            workspacePackages.set(packageJson.name, {
              version: packageJson.version,
              path: packageJsonPath,
              dir: dir,
            });
          }
        } catch (error) {
          console.warn(`Failed to read ${packageJsonPath}:`, error.message);
        }
      }
    }
  }

  return workspacePackages;
}

// Update dependencies in a package.json file
function updateDependencies(packageJsonPath, workspacePackages) {
  let packageJson;

  try {
    packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
  } catch (error) {
    console.warn(`Failed to read ${packageJsonPath}:`, error.message);
    return 0;
  }

  let updatedCount = 0;

  // Dependency types to check
  const dependencyTypes = [
    'dependencies',
    'devDependencies',
    'peerDependencies',
    'optionalDependencies',
  ];

  for (const depType of dependencyTypes) {
    if (packageJson[depType]) {
      for (const [depName, currentVersion] of Object.entries(
        packageJson[depType],
      )) {
        // Check if this dependency is a workspace package
        if (workspacePackages.has(depName)) {
          const targetVersion = workspacePackages.get(depName).version;

          // Skip dependencies set to "*" - don't replace wildcards with explicit versions
          if (currentVersion === '*') {
            continue;
          }

          if (currentVersion !== targetVersion) {
            console.log(
              `${packageJsonPath}: Updating ${depName} from ${currentVersion} to ${targetVersion}`,
            );
            packageJson[depType][depName] = targetVersion;
            updatedCount++;
          }
        }
      }
    }
  }

  // Write back the updated package.json if any changes were made
  if (updatedCount > 0) {
    try {
      fs.writeFileSync(
        packageJsonPath,
        JSON.stringify(packageJson, null, 2) + '\n',
      );
      console.log(`Updated ${updatedCount} dependencies in ${packageJsonPath}`);
    } catch (error) {
      console.error(`Failed to write ${packageJsonPath}:`, error.message);
    }
  }

  return updatedCount;
}

// Main function
function main() {
  console.log('🔍 Finding workspace packages...');
  const workspacePackages = findWorkspacePackages();

  console.log(`\n📦 Found ${workspacePackages.size} workspace packages:`);
  for (const [name, info] of workspacePackages) {
    console.log(`  ${name}@${info.version}`);
  }

  console.log('\n🔧 Finding all package.json files...');
  const allPackageJsonFiles = findAllPackageJsonFiles();
  console.log(`Found ${allPackageJsonFiles.length} package.json files`);

  console.log('\n🔄 Updating dependency versions...');
  let totalUpdates = 0;

  for (const packageJsonPath of allPackageJsonFiles) {
    const updates = updateDependencies(packageJsonPath, workspacePackages);
    totalUpdates += updates;
  }

  console.log(
    `\n✅ Done! Updated ${totalUpdates} dependencies across all files.`,
  );

  if (totalUpdates > 0) {
    console.log(
      '\n💡 Tip: You may want to run "yarn install" to update lock files.',
    );
  }
}

// Run the script
if (require.main === module) {
  main();
}

module.exports = {
  findWorkspacePackages,
  updateDependencies,
  findAllPackageJsonFiles,
};
