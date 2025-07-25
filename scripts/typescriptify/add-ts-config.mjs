/* eslint-disable no-console */
import * as fs from 'node:fs';
import * as url from 'node:url';
import * as path from 'node:path';
import glob from 'glob';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const __root = path.join(__dirname, '..', '..');

for (const packageJsonPathRel of glob.sync('packages/**/*/package.json', {
  cwd: __root,
})) {
  if (packageJsonPathRel.includes('/apvm/')) continue;
  if (packageJsonPathRel.includes('/node_modules/')) continue;
  if (packageJsonPathRel.includes('/fixture/')) continue;
  if (packageJsonPathRel.includes('/fixtures/')) continue;
  if (packageJsonPathRel.includes('/template/')) continue;
  if (packageJsonPathRel.includes('/lib/')) continue;
  if (packageJsonPathRel.includes('/configs/')) continue;
  if (packageJsonPathRel.includes('/core/integration-tests/')) continue;

  const packageJsonPath = path.join(__root, packageJsonPathRel);
  const packagePath = path.dirname(packageJsonPath);
  const pkg = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));

  if (!pkg.scripts) {
    pkg.scripts = {};
  }

  if (pkg.scripts['build-ts']) {
    delete pkg.scripts['build-ts'];
  }

  pkg.scripts['check-ts'] = 'tsc --noEmit';

  if (pkg.source) {
    try {
      pkg.source = pkg.source.replace('.js', '.ts');
      pkg['types'] = pkg.source;
      fs.writeFileSync(packageJsonPath, JSON.stringify(pkg, null, 2), 'utf8');
    } catch (error) {
      console.log('err', pkg.name, packageJsonPath);
    }
  } else {
    console.log('skip', pkg.name, packageJsonPath);
  }

  fs.writeFileSync(
    path.join(packagePath, 'tsconfig.json'),
    JSON.stringify(
      {
        extends: '../../../tsconfig.json',
        include: ['src'],
      },
      null,
      2,
    ),
    'utf8',
  );
}
