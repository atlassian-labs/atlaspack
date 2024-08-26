import {spawnSync as spawn} from 'child_process';

import {minVersion} from 'semver';

import {sortPackageJsonField} from './sort-package-json-field';

const versions = new Map<string, string>();

function isValidRange(range: string): boolean {
  try {
    return !!minVersion(range);
  } catch (err) {
    return false;
  }
}

function getAtlaspackPackageName(parcelPackageName: string): string {
  switch (parcelPackageName) {
    case 'parcel':
      return '@atlaspack/cli';
    default:
      return `@atlaspack/${parcelPackageName.replace('@parcel/', '')}`;
  }
}

function getVersion(name: string, range: string, tag: string): string {
  let version = versions.get(name);
  if (version) {
    return version;
  }

  // If the range is invalid, then just use it as is
  if (!isValidRange(range)) {
    return range;
  }

  const {status, stdout} = spawn(
    'npm',
    ['show', '--loglevel', 'error', `${name}@${tag}`, 'version'],
    {
      stdio: ['pipe', 'pipe', 'inherit'],
    },
  );

  if (status) {
    throw new Error(`Failed to retrieve canary version for ${name}`);
  }

  version = `^${stdout.toString().trim()}`;
  versions.set(name, version);
  return version;
}

export function migrateDependencies(packageJson: any, tag: string): boolean {
  let didDependenciesChange = false;
  const skipPackages = new Set([
    '@parcel/hash',
    '@parcel/source-map',
    '@parcel/watcher',
  ]);

  for (const field of [
    'dependencies',
    'devDependencies',
    'peerDependencies',
    'resolutions',
  ]) {
    if (!packageJson[field]) {
      continue;
    }

    let didDependencyFieldChange = false;
    for (const key of Object.keys(packageJson[field])) {
      const isParcelPackage = key === 'parcel' || key.startsWith('@parcel/');
      if (!isParcelPackage || skipPackages.has(key)) {
        continue;
      }

      const packageName = getAtlaspackPackageName(key);
      const version = packageJson[field][key];
      const nextVersion = getVersion(packageName, version, tag);
      if (version !== nextVersion) {
        packageJson[field][packageName] = nextVersion;
        delete packageJson[field][key];
        didDependencyFieldChange = true;
      }
    }

    if (didDependencyFieldChange) {
      sortPackageJsonField(packageJson, field);
      didDependenciesChange = true;
    }
  }

  return didDependenciesChange;
}
