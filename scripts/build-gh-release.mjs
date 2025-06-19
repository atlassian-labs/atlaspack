/* eslint-disable no-console */

/*
  This script creates a tar-ball of this repo, designed to be consumed with apvm.
*/

import * as path from 'node:path';
import * as fs from 'node:fs';
import * as crypto from 'node:crypto';
import * as process from 'node:process';
import * as child_process from 'node:child_process';
import * as url from 'node:url';
import tmpDir from 'temp-dir';
import fsExtra from 'fs-extra';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const __root = path.dirname(__dirname);

void (async function main() {
  const __tmp = path.join(
    tmpDir,
    `atlaspack-${(Math.random() * 100000000000).toFixed()}`,
  );
  const release = `atlaspack-universal`;
  const releaseRoot = path.join(__root, 'release');
  const releaseDir = path.join(releaseRoot, release);
  const releaseTarGz = path.join(releaseRoot, `${release}.tar.gz`);
  const releaseTarXz = path.join(releaseRoot, `${release}.tar.xz`);

  // Create release dir
  createOrReplaceDir(releaseDir);
  fs.rmSync(releaseTarGz, {
    recursive: true,
    force: true,
  });
  fs.rmSync(releaseTarXz, {
    recursive: true,
    force: true,
  });

  console.log('Temp Dir', __tmp);
  createOrReplaceDir(__tmp);

  console.log('Copying to temp directory');
  try {
    // fs.cpSync(path.join(__root, 'packages'), path.join(__tmp, 'packages'). { recursive: true })
    fs.cpSync(
      path.join(__root, 'package.json'),
      path.join(__tmp, 'package.json'),
      {recursive: true},
    );

    fs.cpSync(path.join(__root, 'yarn.lock'), path.join(__tmp, 'yarn.lock'), {
      recursive: true,
    });

    createOrReplaceDir(path.join(__tmp, 'packages'));
    await fsExtra.copy(path.join(__root, 'packages'), path.join(__tmp, 'packages'), {
      filter: (path) => {
        if (path.includes('.git')) return false;
        if (path.includes('fixture')) return false;
        if (path.includes('release')) return false;
        if (path.includes('target')) return false;
        if (path.includes('node_modules')) return false;
        if (path.endsWith('.gitignore')) return false;
        if (fs.lstatSync(path).isFile()) return true;
        if (fs.lstatSync(path).isDirectory()) return true;
        return false;
      },
    });

    fs.rmSync(path.join(__tmp, 'packages', 'examples'), { recursive: true, force: true });
    fs.rmSync(path.join(__tmp, 'packages', 'dev'), { recursive: true, force: true });
    fs.rmSync(path.join(__tmp, 'packages', 'core', 'e2e-tests'), { recursive: true, force: true });
    fs.rmSync(path.join(__tmp, 'packages', 'core', 'integration-tests'), { recursive: true, force: true });
    fs.rmSync(path.join(__tmp, 'packages', 'core', 'test-utils'), { recursive: true, force: true });

    console.log('Installing dependencies');
    child_process.execFileSync(
      'yarn',
      ['install', '--ignore-platform', '--ignore-engines'],
      {
        stdio: 'inherit',
        shell: true,
        cwd: __tmp,
        env: {
          ...process.env,
          NODE_ENV: 'production'
        }
      },
    );

    console.log('copying output');
    await fsExtra.copy(__tmp, releaseDir);

    console.log('Creating tarball (gz)');
    child_process.execFileSync('tar', ['-czf', releaseTarGz, '.'], {
      stdio: 'inherit',
      shell: true,
      cwd: __tmp,
    });
    const hashGz = await calcHash(releaseTarGz)
    console.log(hashGz)
    writeFile(
      path.join(releaseRoot, `${release}.tar.gz.integrity`),
      `sha512-${hashGz}`,
    );

    console.log('Creating tarball (xz)');
    child_process.execFileSync('tar', ['-cJf', releaseTarXz, '.'], {
      stdio: 'inherit',
      shell: true,
      cwd: __tmp,
    });
    const hashXz = await calcHash(releaseTarXz)
    console.log(hashXz)
    writeFile(
      path.join(releaseRoot, `${release}.tar.xz.integrity`),
      `sha512-${hashXz}`,
    );
  } finally {
    console.log('Cleanup');

    fs.rmSync(__tmp, {
      recursive: true,
      force: true,
    });
  }
})();

// -----
// Utils
// -----
function calcHash(target) {
  return new Promise((resolve, reject) => {
    const hash = crypto.createHash('sha512');
    const stream = fs.createReadStream(target);
    stream.on('error', (err) => reject(err));
    stream.on('data', (chunk) => hash.update(chunk));
    stream.on('end', () => resolve(hash.digest('base64')));
  });
}

function removeDirAll(target) {
  if (fs.existsSync(target)) {
    fs.rmSync(target, {
      recursive: true,
      force: true,
    });
  }
}

function createOrReplaceDir(target) {
  removeDirAll(target);
  fs.mkdirSync(target, {recursive: true});
}

function writeFile(target, data) {
  fs.writeFileSync(target, data, 'utf8');
}

function writeJson(target, obj) {
  writeFile(target, JSON.stringify(obj, null, 2));
}

function readFile(target) {
  return fs.readFileSync(target, 'utf8');
}

function readJson(target) {
  return JSON.parse(readFile(target));
}

function sortObject(input) {
  return Object.keys(input)
    .sort()
    .reduce((obj, key) => {
      obj[key] = input[key];
      return obj;
    }, {});
}
