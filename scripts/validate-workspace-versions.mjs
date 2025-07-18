/* eslint-disable no-console */
// This package goes through an ensures all versions of local packages
// in the workspace package.json files point to the local version of those
// packages.
//
// This avoids scenarios where merges/rebases mess up the "package.json#dependencies"
// and installs the workspace packages from npm
import * as path from 'node:path';
import * as fs from 'node:fs';
import * as url from 'node:url';
import glob from 'glob';

const dirname = path.dirname(url.fileURLToPath(import.meta.url));
const root = path.dirname(dirname);

const rootPkg = JSON.parse(
  fs.readFileSync(path.join(root, 'package.json'), 'utf8'),
);

const packages = new Map();
const packagesVersions = new Map();

for (const workspace of rootPkg.workspaces) {
  for (const pkgPath of glob.sync(`${workspace}/package.json`, {cwd: root})) {
    const abs = path.join(root, pkgPath);
    const pkg = JSON.parse(fs.readFileSync(abs, 'utf8'));
    packages.set(abs, pkg);
    packagesVersions.set(pkg.name, pkg.version);
  }
}

for (const [pkgPath, pkg] of packages.entries()) {
  for (const [packageName, version] of Object.entries(pkg.dependencies || {})) {
    const current = packagesVersions.get(packageName);
    if (current && version !== '*' && version != current) {
      console.log(
        `Miss\n\tExpected ${packageName}@${version}\n\tGot      ${packageName}@${current}\n\t${pkgPath}`,
      );
    }
  }

  for (const [packageName, version] of Object.entries(pkg.devDependencies || {})) {
    const current = packagesVersions.get(packageName);
    if (current && version !== '*' && version != current) {
      console.log(
        `Miss\n\tExpected ${packageName}@${version}\n\tGot      ${packageName}@${current}\n\t${pkgPath}`,
      );
    }
  }

  for (const [packageName, version] of Object.entries(pkg.optionalDependencies || {})) {
    const current = packagesVersions.get(packageName);
    if (current && version !== '*' && version != current) {
      console.log(
        `Miss\n\tExpected ${packageName}@${version}\n\tGot      ${packageName}@${current}\n\t${pkgPath}`,
      );
    }
  }
}
