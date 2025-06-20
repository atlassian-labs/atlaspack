#!/usr/bin/env node

import * as child_process from 'node:child_process';
import * as process from 'node:process';
import * as fs from 'node:fs';
import * as path from 'node:path';
import * as url from 'node:url';

const arch = {
  x64: 'amd64',
  arm64: 'arm64',
}[process.arch];

const platform = {
  linux: 'linux',
  darwin: 'macos',
  win32: 'windows',
}[process.platform];

let binPath = process.env.APVM_BIN_PATH;

if (!binPath) {
  const packageJsonPath = url.fileURLToPath(
    import.meta.resolve(`@atlaspack/apvm-${platform}-${arch}/package.json`),
  );
  const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
  const packageJsonDir = path.dirname(packageJsonPath);
  binPath = path.join(
    packageJsonDir,
    packageJson.bin[`apvm-${platform}-${arch}`],
  );
}

if (!fs.existsSync(binPath)) {
  // eslint-disable-next-line no-console
  console.error(`BinaryDoesNotExist: ${binPath}`);
  process.exit(1);
}

if (platform !== 'windows' && !isExec(binPath)) {
  fs.chmodSync(binPath, '755');
}

const [, , ...args] = process.argv;
try {
  child_process.execFileSync(binPath, args, {
    stdio: 'inherit',
    shell: true,
  });
} catch (err) {
  process.exit(err.status);
}

function isExec(p) {
  return !!(fs.statSync(p).mode & fs.constants.S_IXUSR);
}
