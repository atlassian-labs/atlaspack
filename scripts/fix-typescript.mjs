import {glob, fs, path} from 'zx';

const allPackages = await glob('packages/**/package.json');

for (let packagePath of allPackages) {
  if (
    packagePath.includes('integration-tests') ||
    packagePath.includes('/fixture/')
  ) {
    continue;
  }

  const packageJson = JSON.parse(fs.readFileSync(packagePath, 'utf8'));
  if (packageJson.source) {
    const jsExists = fs.existsSync(
      path.join(path.dirname(packagePath), packageJson.source),
    );
    const tsSource = path.join(
      path.dirname(packageJson.source),
      path.basename(packageJson.source, '.js') + '.ts',
    );
    const tsExists = fs.existsSync(
      path.join(path.dirname(packagePath), tsSource),
    );
    if (!jsExists && tsExists) {
      packageJson.source = tsSource;
      fs.writeFileSync(packagePath, JSON.stringify(packageJson, null, 2));
    }
  }

  if (packageJson.main) {
    const jsExists = fs.existsSync(
      path.join(path.dirname(packagePath), packageJson.main),
    );
    const tsmain = path.join(
      path.dirname(packageJson.main),
      path.basename(packageJson.main, '.js') + '.ts',
    );
    const tsExists = fs.existsSync(
      path.join(path.dirname(packagePath), tsmain),
    );
    if (!jsExists && tsExists) {
      packageJson.main = tsmain;
      fs.writeFileSync(packagePath, JSON.stringify(packageJson, null, 2));
    }
  }
}
