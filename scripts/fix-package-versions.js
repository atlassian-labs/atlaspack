/* eslint-disable no-console */
import * as fs from 'node:fs';
import * as url from 'node:url';
import * as path from 'node:path';
import glob from 'glob';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const __root = path.join(__dirname, '..');

const IGNORED_PATTERNS = [
  'core/atlaspack',
  'apvm',
  'node_modules',
  'node-resolver-core/test/fixture',
  'test/fixtures',
  'examples',
  'integration-tests',
  'workers/test/integration',
];

// Build map of internal package versions (@atlaspack/* -> version)
const packageVersionMap = new Map();

for (const packageJsonPathRel of glob.sync('packages/**/*/package.json', {
  cwd: __root,
})) {
  if (
    IGNORED_PATTERNS.some((pattern) => packageJsonPathRel.includes(pattern))
  ) {
    continue;
  }

  console.log(packageJsonPathRel);
  const packageJsonAbsPath = path.join(__root, packageJsonPathRel);
  const packageJson = JSON.parse(fs.readFileSync(packageJsonAbsPath, 'utf8'));

  if (packageJson.name?.startsWith('@atlaspack/')) {
    packageVersionMap.set(packageJson.name, packageJson.version);
  }
}

// Iterate again and update dependency specs to latest internal versions
for (const packageJsonPathRel of glob.sync('packages/**/*/package.json', {
  cwd: __root,
})) {
  if (
    IGNORED_PATTERNS.some((pattern) => packageJsonPathRel.includes(pattern))
  ) {
    continue;
  }

  const packageJsonAbsPath = path.join(__root, packageJsonPathRel);
  const packageJson = JSON.parse(fs.readFileSync(packageJsonAbsPath, 'utf8'));

  let changed = false;

  for (const depField of [
    'dependencies',
    'devDependencies',
    'peerDependencies',
    'optionalDependencies',
  ]) {
    const deps = packageJson[depField];
    if (!deps) continue;

    for (const [depName, currentSpec] of Object.entries(deps)) {
      if (!depName.startsWith('@atlaspack/')) continue;

      const correctVersion = packageVersionMap.get(depName);
      if (!correctVersion) {
        console.warn(`Could not find version for ${depName}`);
        continue;
      }

      // Strip common prefixes when comparing
      const stripped = currentSpec
        .replace(/^workspace:/, '')
        .replace(/^[\^~]/, '');

      if (stripped !== correctVersion) {
        const prefix = currentSpec.startsWith('workspace:') ? 'workspace:' : '';
        deps[depName] = `${prefix}${correctVersion}`;
        changed = true;
        console.log(
          `${packageJson.name ?? packageJsonPathRel}: ${depName} ${currentSpec} -> ${deps[depName]}`,
        );
      }
    }
  }

  if (changed) {
    fs.writeFileSync(
      packageJsonAbsPath,
      `${JSON.stringify(packageJson, null, 2)}\n`,
    );
  }
}
