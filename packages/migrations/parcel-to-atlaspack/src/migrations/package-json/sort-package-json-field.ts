export function sortPackageJsonField(packageJson: any, field: string) {
  const value = {...packageJson[field]};
  packageJson[field] = {};
  for (const key of Object.keys(value).sort()) {
    packageJson[field][key] = value[key];
  }
}
