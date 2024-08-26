// TODO Preserve order?
export function migrateConfigFields(packageJson: any): boolean {
  let didConfigChange = false;

  for (const field of Object.keys(packageJson).filter(key =>
    key.startsWith('@parcel/'),
  )) {
    packageJson[`@atlaspack/${field.replace('@parcel/', '')}`] =
      packageJson[field];
    delete packageJson[field];
    didConfigChange = true;
  }

  return didConfigChange;
}
