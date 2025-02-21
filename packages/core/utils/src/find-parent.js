// @flow

import fs from 'fs';
import path from 'path';

export function findParent(start: string, lookFor: string): ?string {
  if (fs.existsSync(path.join(start, lookFor))) {
    return path.join(start, lookFor);
  }
  const parentDir = path.resolve(start, '..');
  if (start === parentDir) {
    return null;
  }
  return findParent(parentDir, lookFor);
}

export function findPackageJson(start: string): Object {
  const target = findParent(start, 'package.json');
  if (!target) {
    throw new Error('Unable to find package.json');
  }
  return JSON.parse(fs.readFileSync(target, 'utf8'));
}
