import * as fs from 'node:fs';
import * as url from 'node:url';
import * as path from 'node:path';
import {execFileSync} from 'node:child_process';

const __filename = url.fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const __package = path.normalize(path.join(__dirname, '..'));

if (fs.existsSync(path.join(__package, 'GIT_SHA'))) {
  fs.rmSync(path.join(__package, 'GIT_SHA'));
}

const result = execFileSync('git', ['rev-parse', 'HEAD'], {
  shell: true,
  stdio: 'pipe',
});

fs.writeFileSync(path.join(__package, 'GIT_SHA'), `${result}`.trim(), 'utf8');
