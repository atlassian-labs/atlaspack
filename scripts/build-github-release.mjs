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
import glob from 'glob';
import fsExtra from 'fs-extra';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const __root = path.dirname(__dirname);
const __tmp = path.join(
  tmpDir,
  `atlaspack-${(Math.random() * 100000000000).toFixed()}`,
);

void (async function main() {
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
    await fsExtra.copy(
      path.join(__root, 'packages'),
      path.join(__tmp, 'packages'),
      {
        filter: (fullPath) => {
          const stat = fs.lstatSync(fullPath);
          const relPath = fullPath.replace(__root + path.sep, '');
          if (stat.isDirectory() && relPath.includes('.git')) return false;
          if (stat.isDirectory() && relPath.includes('fixture')) return false;
          if (stat.isDirectory() && relPath.includes('release')) return false;
          if (stat.isDirectory() && relPath.includes('node_modules'))
            return false;
          if (stat.isDirectory() && relPath.startsWith('target')) return false;
          if (stat.isFile() && relPath.endsWith('.gitignore')) return false;
          if (stat.isFile()) return true;
          if (stat.isDirectory()) return true;
          return false;
        },
      },
    );

    fs.rmSync(path.join(__tmp, 'packages', 'examples'), {
      recursive: true,
      force: true,
    });
    fs.rmSync(path.join(__tmp, 'packages', 'apvm'), {
      recursive: true,
      force: true,
    });
    fs.rmSync(path.join(__tmp, 'packages', 'dev'), {
      recursive: true,
      force: true,
    });
    fs.rmSync(path.join(__tmp, 'packages', 'core', 'e2e-tests'), {
      recursive: true,
      force: true,
    });
    fs.rmSync(path.join(__tmp, 'packages', 'core', 'integration-tests'), {
      recursive: true,
      force: true,
    });
    fs.rmSync(path.join(__tmp, 'packages', 'core', 'test-utils'), {
      recursive: true,
      force: true,
    });

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
          NODE_ENV: 'production',
        },
      },
    );

    if (process.env.VERSION !== undefined) {
      console.log('Updating versions', process.env.VERSION);
      for (const entry of glob.sync('packages/**/package.json', {cwd: __tmp})) {
        const fullPath = path.join(__tmp, entry);
        const json = readJson(fullPath);
        if (!json.name?.startsWith('@atlaspack')) continue;

        json.version = `2.0.0-${process.env.VERSION}`;
        json.releaseInfo = {
          date: process.env.DATE,
          sha: process.env.SHA,
        };

        json.dependencies = json.dependencies || {};
        for (const key of Object.keys(json.dependencies)) {
          if (key.startsWith('@atlaspack')) {
            json.dependencies[key] = json.version;
          }
        }

        json.devDependencies = undefined;

        json.peerDependencies = json.peerDependencies || {};
        for (const key of Object.keys(json.peerDependencies)) {
          if (key.startsWith('@atlaspack')) {
            json.peerDependencies[key] = json.version;
          }
        }

        json.optionalDependencies = json.optionalDependencies || {};
        for (const key of Object.keys(json.optionalDependencies)) {
          if (key.startsWith('@atlaspack')) {
            json.optionalDependencies[key] = json.version;
          }
        }

        json.scripts = undefined;
        json.engines = undefined;
        json.source = undefined;

        writeJson(fullPath, json);
      }
    }

    console.log('copying output');
    await fsExtra.copy(__tmp, releaseDir);

    console.log('Creating tarball (gz)');
    child_process.execFileSync('tar', ['-czf', releaseTarGz, '.'], {
      stdio: 'inherit',
      shell: true,
      cwd: __tmp,
    });
    writeFile(
      path.join(releaseRoot, `${release}.tar.gz.sha512`),
      `sha512-${await calcHash(releaseTarGz)}`,
    );

    console.log('Creating tarball (xz)');
    child_process.execFileSync('tar', ['-cJf', releaseTarXz, '.'], {
      stdio: 'inherit',
      shell: true,
      cwd: __tmp,
    });
    writeFile(
      path.join(releaseRoot, `${release}.tar.xz.sha512`),
      `sha512-${await calcHash(releaseTarXz)}`,
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

function readFile(target) {
  return fs.readFileSync(target, 'utf8');
}

function writeJson(target, obj) {
  writeFile(target, JSON.stringify(obj, null, 2));
}

function readJson(target) {
  return JSON.parse(readFile(target));
}
