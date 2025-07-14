import * as fs from 'node:fs';
import * as url from 'node:url';
import * as path from 'node:path';
import {execFileSync} from 'node:child_process';
import glob from 'glob';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const __root = path.dirname(__dirname);

for (const entry of glob.sync('packages/*/*/lib', {
  cwd: __root,
  ignore: ['.git', 'node_modules'],
})) {
  execFileSync('git', ['clean', '-xdf', entry], {
    cwd: __root,
    shell: true,
    stdio: 'inherit',
  });
}
