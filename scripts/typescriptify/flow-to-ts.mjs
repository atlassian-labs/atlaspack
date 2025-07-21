import * as fs from 'node:fs';
import * as url from 'node:url';
import * as path from 'node:path';
import {execFileSync} from 'node:child_process';
import glob from 'glob';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const __root = path.join(__dirname, '..', '..');

const todo = new Set();

// Find the flow fliles
for (const packageJsonPathRel of glob.sync('packages/**/*/package.json', {
  cwd: __root,
})) {
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

  const packageJsonPath = path.join(__root, packageJsonPathRel);
  const packagePath = path.dirname(packageJsonPath);

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

const gitignores = [
  'packages/core/diagnostic/.gitignore',
  'packages/core/profiler/.gitignore',
];

for (const gitignore of gitignores) {
  fs.rmSync(path.join(__root, gitignore));
}

// Convert the files
execFileSync(
  'node',
  [
    path.join(__dirname, 'tmp', 'flow-to-typescript-codemod', 'bin.js'),
    'convert',
    '--autoSuppressErrors',
    '--write',
    '--delete',
    ...Array.from(todo).flatMap((p) => ['-p', p]),
  ],
  {
    stdio: 'inherit',
    shell: true,
    cwd: __root,
  },
);
