/* eslint-disable no-console */
import * as path from 'node:path';
import * as fs from 'node:fs';
import * as child_process from 'node:child_process';
import * as url from 'node:url';
import glob from 'glob';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const __root = path.dirname(__dirname);
const __src = path.join(__root, 'packages', 'unified', 'src');
const __dist = path.join(__root, 'packages', 'unified', 'lib');

const bins = {
  flowToTs: path.join(__root, 'node_modules', '.bin', 'flow-to-ts'),
  tsc: path.join(__root, 'node_modules', '.bin', 'tsc'),
};

void (async function main() {
  const inFlight = [];

  for (const foundRel of glob.sync('**/*.js', {
    cwd: __src,
    ignore: ['**/*.test.js', '**/node_modules/**', '**/vendor/**'],
  })) {
    const task = doAsync(async () => {
      const input = path.join(__src, foundRel);
      const outputTypeScript = path
        .join(__dist, foundRel)
        .replace('.js', '.ts');

      if (!fs.existsSync(path.dirname(outputTypeScript))) {
        await fs.promises.mkdir(path.dirname(outputTypeScript));
      }

      const result = await spawn('node', [bins.flowToTs, input], {
        cwd: __src,
        shell: true,
      });

      await fs.promises.writeFile(outputTypeScript, result, 'utf8');

      await spawn(
        'node',
        [
          bins.tsc,
          '--emitDeclarationOnly',
          '--declaration',
          '--esModuleInterop',
          outputTypeScript,
        ],
        {
          cwd: __src,
          shell: true,
        },
      );

      await fs.promises.rm(outputTypeScript);
    });

    inFlight.push(task);
  }

  await Promise.all(inFlight);
})();

function doAsync(fn) {
  return new Promise(
    (res, rej) =>
      setTimeout(async () => {
        try {
          res(await fn());
        } catch (error) {
          rej(error);
        }
      }),
    0,
  );
}

function spawn(cmd, args, options) {
  return new Promise((resolve, reject) => {
    const cp = child_process.spawn(cmd, args, options);
    const error = [];
    const stdout = [];
    cp.stdout.on('data', (data) => {
      stdout.push(data.toString());
    });

    cp.on('error', (e) => {
      error.push(e.toString());
    });

    cp.on('close', () => {
      if (error.length) reject(error.join(''));
      else resolve(stdout.join(''));
    });
  });
}
