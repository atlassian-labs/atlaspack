import * as fs from 'node:fs';
import * as url from 'node:url';
import * as path from 'node:path';
import {execFileSync} from 'node:child_process';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));

// Clone the flow-to-ts codemod
if (fs.existsSync(path.join(__dirname, 'tmp'))) {
  fs.rmSync(path.join(__dirname, 'tmp'), {recursive: true, force: true});
}
fs.mkdirSync(path.join(__dirname, 'tmp'));

execFileSync(
  'git',
  [
    'clone',
    '--depth=1',
    'https://github.com/stripe-archive/flow-to-typescript-codemod.git',
    path.join(__dirname, 'tmp', 'flow-to-typescript-codemod'),
  ],
  {
    stdio: 'inherit',
    shell: true,
    cwd: path.join(__dirname, 'tmp'),
  },
);
fs.rmSync(path.join(__dirname, 'tmp', 'flow-to-typescript-codemod', '.git'), {
  recursive: true,
  force: true,
});

execFileSync('yarn', {
  stdio: 'inherit',
  shell: true,
  cwd: path.join(__dirname, 'tmp', 'flow-to-typescript-codemod'),
});
execFileSync('yarn', ['build'], {
  stdio: 'inherit',
  shell: true,
  cwd: path.join(__dirname, 'tmp', 'flow-to-typescript-codemod'),
});
