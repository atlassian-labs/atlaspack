import * as url from 'node:url';
import * as path from 'node:path';
import {execFileSync} from 'node:child_process';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const __root = path.join(__dirname, '..', '..');

execFileSync('npx', ['eslint', '.', '--fix'], {
  stdio: 'inherit',
  shell: true,
  cwd: __root,
});

execFileSync('npx', ['prettier', '--write', '.'], {
  stdio: 'inherit',
  shell: true,
  cwd: __root,
});
