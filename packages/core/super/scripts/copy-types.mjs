/* eslint-disable import/no-extraneous-dependencies */

// This will copy types over to the `/types` folder and rewrite imports to relative paths

import * as fs from 'node:fs';
import * as path from 'node:path';
import * as url from 'node:url';
import glob from 'glob';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const __types = path.normalize(path.join(__dirname, '..', 'types'));
const __root = path.normalize(path.join(__dirname, '..', '..', '..', '..'));

if (fs.existsSync(__types)) {
  fs.rmSync(__types, {recursive: true, force: true});
}
fs.mkdirSync(__types);

const packageJsonFiles = glob.sync('**/*/package.json', {
  cwd: path.join(__root, 'packages'),
  ignore: [
    '**/node_modules/**',
    '**/integration-tests/**',
    '**/test/**',
    '**/examples/**',
  ],
});

const typePaths = {};

// Analyze repo to resolve paths to "package.json#types"
for (const pkg of packageJsonFiles) {
  const pkgDir = path.dirname(pkg);

  const pkgJson = JSON.parse(
    fs.readFileSync(path.join(__root, 'packages', pkg), 'utf8'),
  );

  // You cannot import "d.ts" extensions, must import ".js"
  if (pkgJson.types) {
    typePaths[pkgJson.name] = path
      .join(__types, pkgDir, pkgJson.types)
      .replace('.d.ts', '.js');
  } else if (
    fs.existsSync(path.join(__root, 'packages', pkgDir, 'index.d.ts'))
  ) {
    typePaths[pkgJson.name] = path.join(__types, pkgDir, 'index.js');
  }
}

// Copy types into "/types" and rewrite imports to be relative paths
for (const pkg of packageJsonFiles) {
  const pkgDir = path.dirname(pkg);

  const found = glob.sync('**/*.d.ts', {
    cwd: path.join(__root, 'packages', pkgDir),
    ignore: ['**/node_modules/**'],
  });

  if (!found.length) {
    continue;
  }

  for (const entry of found) {
    let entryPath = path.join(__types, pkgDir, entry);
    let entryDir = path.dirname(entryPath);
    if (!fs.existsSync(entryDir)) {
      fs.mkdirSync(entryDir, {recursive: true});
    }

    let content = fs.readFileSync(
      path.join(__root, 'packages', pkgDir, entry),
      'utf8',
    );

    for (const [sourceKey, newPath] of Object.entries(typePaths)) {
      content = content.replaceAll(sourceKey, path.relative(entryDir, newPath));
    }

    fs.writeFileSync(entryPath, content, 'utf8');
  }
}
