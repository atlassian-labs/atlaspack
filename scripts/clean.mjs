import * as url from 'node:url';
import * as path from 'node:path';
import {execFileSync} from 'node:child_process';
import glob from 'glob';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const __root = path.dirname(__dirname);

const patterns = [
  'packages/*/*/{lib,dist}',
  'packages/**/tsconfig.tsbuildinfo',
];

for (const pattern of patterns) {
  const entries = glob.sync(pattern, {
    cwd: __root,
    ignore: ['.git', 'node_modules'],
  });

  for (const entry of entries) {
    execFileSync('git', ['clean', '-xdf', entry], {
      cwd: __root,
      shell: true,
      stdio: 'inherit',
    });
  }
}
