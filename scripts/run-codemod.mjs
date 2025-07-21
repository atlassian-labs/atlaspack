import * as fs from 'node:fs';
import * as path from 'node:path';
import {execFileSync, execSync} from 'node:child_process';
import glob from 'glob';

const startTime = Date.now();

const todo = new Set();
const gitignores = [];

for (const packageJsonPathRel of glob.sync('packages/**/*/package.json', {})) {
  console.log(packageJsonPathRel);
  if (packageJsonPathRel.includes('/apvm/')) continue;
  if (packageJsonPathRel.includes('/core/atlaspack/')) continue;
  if (packageJsonPathRel.includes('/node_modules/')) continue;
  if (packageJsonPathRel.includes('/fixture/')) continue;
  if (packageJsonPathRel.includes('/fixtures/')) continue;
  if (packageJsonPathRel.includes('/template/')) continue;
  if (packageJsonPathRel.includes('/lib/')) continue;
  if (packageJsonPathRel.includes('/core/configs/')) continue;
  if (packageJsonPathRel.includes('/core/integration-tests/integration'))
    continue;
  if (packageJsonPathRel.includes('/core/integration-tests/test/integration/'))
    continue;
  const packageJsonPath = path.join(process.cwd(), packageJsonPathRel);
  const packagePath = path.dirname(packageJsonPath);
  if (fs.existsSync(path.join(packagePath, '.gitignore'))) {
    gitignores.push(path.join(packagePath, '.gitignore'));
  }
  for (const source of glob.sync('**/*.js', {cwd: packagePath})) {
    const sourcePath = path.join(packagePath, source);
    if (sourcePath.includes('/lib/')) continue;
    if (sourcePath.includes('/core/integration-tests/test/integration/'))
      continue;
    const file = fs.readFileSync(sourcePath, 'utf8');
    if (
      !(
        file.startsWith('// @flow') ||
        file.startsWith('//@flow') ||
        file.startsWith('//  @flow')
      )
    ) {
      continue;
    }
    todo.add(sourcePath);
  }
}

flowToTsSync(...Array.from(todo));
for (const gitignore of gitignores) console.log(gitignore);

const pkgs = [];
for (const packageJsonPathRel of glob.sync('packages/**/*/package.json')) {
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
  if (!pkg.scripts['build-ts']) {
    delete pkg.scripts['build-ts'];
  }
  pkg.scripts['check-ts'] = 'tsc --noEmit';
  if (pkg.source) {
    try {
      pkg.source = pkg.source.replace('.js', '.ts');
      pkg['types'] = pkg.source;
      fs.writeFileSync(packageJsonPath, JSON.stringify(pkg, null, 2), 'utf8');
      execFileSync('npx', ['sort-package-json', packageJsonPath], {
        cwd: __root,
        shell: true,
        stdio: 'inherit',
      });
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

console.log((Date.now() - startTime) / 1000);

function flowToTsSync(...sourcePath) {
  execSync('yarn', {
    cwd: path.join(process.cwd(), 'flow-to-typescript-codemod'),
  });
  execFileSync(
    'yarn',
    [
      'typescriptify',
      'convert',
      '--autoSuppressErrors',
      '--write',
      '--delete',
      ...sourcePath.flatMap((p) => ['-p', p]),
    ],
    {
      stdio: 'inherit',
      shell: true,
      cwd: path.join(process.cwd(), 'flow-to-typescript-codemod'),
    },
  );
}
