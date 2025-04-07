// @flow

import * as path from 'path';
import * as fs from 'fs';

export function findPackageJson(from: string): string | null {
  let currentDir = path.resolve(from);

  // eslint-disable-next-line no-constant-condition
  while (true) {
    const pkgfile = path.join(currentDir, 'package.json');
    if (fs.existsSync(pkgfile)) {
      return pkgfile;
    }
    const dir = path.dirname(currentDir);
    if (dir === currentDir) {
      break;
    }
    currentDir = dir;
  }

  return null;
}

export function readPackageJsonSync<T>(from: string): T {
  const packageJson = findPackageJson(from);
  if (!packageJson) {
    // Cannot return T | null because there are too many failing null checks
    // $FlowFixMe
    return null;
  }
  return JSON.parse(fs.readFileSync(packageJson, 'utf8'));
}
