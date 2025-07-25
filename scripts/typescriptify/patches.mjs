import * as fs from 'node:fs';
import * as url from 'node:url';
import * as path from 'node:path';
import {execFileSync} from 'node:child_process';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const __root = path.join(__dirname, '..', '..');

for (const patch of fs.readdirSync(path.join(__dirname, 'patches'))) {
  execFileSync('git', ['apply', path.join(__dirname, './patches', patch)], {
    stdio: 'inherit',
    shell: true,
    cwd: __root,
  });
}
