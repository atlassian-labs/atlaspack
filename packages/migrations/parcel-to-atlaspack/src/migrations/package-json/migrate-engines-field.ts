import {sortPackageJsonField} from './sort-package-json-field';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function migrateEnginesField(packageJson: any): boolean {
  if (!packageJson.engines?.parcel) {
    return false;
  }

  const version = packageJson.engines.parcel;
  delete packageJson.engines.parcel;
  packageJson.engines.atlaspack = version;

  sortPackageJsonField(packageJson, 'engines');
  return true;
}
