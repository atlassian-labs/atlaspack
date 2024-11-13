/* eslint-disable no-console */

import {execSync as $} from 'node:child_process';
import {
  appendFile,
  cp,
  mkdtemp,
  readdir,
  readFile,
  rm,
  writeFile,
} from 'node:fs/promises';
import {createRequire} from 'node:module';
import {tmpdir} from 'node:os';
import {dirname, join} from 'node:path';

import {Atlaspack} from '@atlaspack/core';
import {printTable} from '@oclif/table';
import chalk from 'chalk';

let headers = 0;

const MODE = process.env.ATLASPACK_BENCH_MODE;
if (MODE === undefined) {
  console.error('env:ATLASPACK_BENCH_MODE not specified');
  process.exit(1);
}

const PLUGINS = process.env.ATLASPACK_BENCH_PLUGINS
  ? parseInt(process.env.ATLASPACK_BENCH_PLUGINS, 10)
  : undefined;

if (PLUGINS === undefined) {
  console.error('env:ATLASPACK_BENCH_USE_PLUGINS not specified');
  process.exit(1);
}

const COPIES =
  process.env.ATLASPACK_BENCH_COPIES !== undefined
    ? parseInt(process.env.ATLASPACK_BENCH_COPIES, 10)
    : 30;

const __dirname = import.meta.dirname;

const RUNS = process.env.ATLASPACK_BENCH_RUNS
  ? parseInt(process.env.ATLASPACK_BENCH_RUNS, 10)
  : 10;

const FUNCTION = process.env.ATLASPACK_BENCH_FUNCTION ?? 'run';

async function main() {
  writeHeader('Settings');

  printTable({
    columns: [
      {key: 'key', name: 'Key      '},
      {
        key: 'value',
        name: 'Value     ',
      },
    ],
    data: [
      {
        key: 'mode',
        value: chalk.green(`'${MODE}'`),
      },
      {
        key: 'function',
        value: chalk.green(`'${FUNCTION}'`),
      },
      {
        key: 'plugins',
        value: chalk.yellow(PLUGINS),
      },
      {
        key: 'copies',
        value: chalk.yellow(COPIES),
      },
      {
        key: 'runs',
        value: chalk.yellow(RUNS),
      },
    ],
    headerOptions: {
      formatter: 'capitalCase',
    },
  });

  writeHeader('Setup');

  // Atlaspack fails to build in the current directory because it is getting
  // settings from the workspace package.json. To get around this, this script
  // copies the benchmark to a temporary directory and links Atlaspack in
  let tmpDir;
  try {
    tmpDir = await mkdtemp(join(tmpdir(), 'atlaspack-bench'));
  } catch (error) {
    console.error('Failed to create temp directory for benchmark', error);
    process.exit(1);
  }

  console.log('Created temporary directory', tmpDir);

  const benchDir = join(__dirname, '..');

  try {
    // Copy files to a temporary directory
    console.log('Copying benchmark...');

    await Promise.all([
      rmrf(join(benchDir, 'dist')),
      rmrf(join(benchDir, '.parcel-cache')),
    ]);

    await cp(benchDir, tmpDir, {recursive: true});
    await rmrf(join(tmpDir, 'node_modules'));

    const [packageJson, parcelRc] = await Promise.all([
      readJson(join(tmpDir, 'package.json')),
      readJson(join(tmpDir, '.parcelrc')),
    ]);

    // Patch the package.json to link the files to the workspace files
    const require = createRequire(import.meta.url);
    for (const dependency of Object.keys(packageJson.dependencies)) {
      if (!dependency.startsWith('@atlaspack')) continue;
      const resolved = require.resolve(join(dependency, 'package.json'));
      packageJson.dependencies[dependency] = `file:${dirname(resolved)}`;
    }

    // Patch .parcelrc to include plugins
    for (let i = 0; i < PLUGINS; i++) {
      parcelRc['transformers']['*.{js,mjs,jsm,jsx,es6,cjs,ts,tsx}'].push(
        './plugins/transformer.js',
      );
    }

    await Promise.all([
      writeJson(join(tmpDir, 'package.json'), packageJson),
      writeJson(join(tmpDir, '.parcelrc'), parcelRc),
    ]);

    // Get three-js
    if ((await readdir(join(benchDir, 'three-js')).length) === 0) {
      console.log('Pulling three-js...');
      $('git submodule update --init ./three-js', {
        cwd: benchDir,
        shell: true,
      });
    }

    // Copy three-js to bench directory
    console.log('Copying sources...');

    const code = [];
    const copies = [];
    const imports = [];

    for (let i = 0; i < COPIES; i++) {
      copies.push(
        cp(
          join(benchDir, 'three-js', 'src'),
          join(tmpDir, 'src', `copy-${i}`),
          {
            recursive: true,
          },
        ),
      );

      imports.push(`import * as three_js_${i} from './copy-${i}/Three.js';`);
      code.push(`globalThis['three_js_${i}'] = three_js_${i};`);
    }

    await Promise.all([
      ...copies,
      appendFile(
        join(tmpDir, 'src', 'index.js'),
        [...imports, ...code].join('\n'),
        'utf8',
      ),
    ]);

    // Link node_modules
    console.log('Linking node_modules...');
    $('npm install', {cwd: tmpDir, shell: true, stdio: 'inherit'});

    // Start the benchmark
    writeHeader('Running');

    const buildTimes = [];

    for (let i = 0; i < RUNS; i++) {
      const startTime = Date.now();

      const atlaspack = new Atlaspack({
        shouldDisableCache: true,
        cacheDir: join(tmpDir, '.parcel-cache'),
        config: join(tmpDir, '.parcelrc'),
        entries: join(tmpDir, 'src', 'index.js'),
        targets: {
          default: {
            distDir: join(tmpDir, 'dist'),
          },
        },
        shouldAutoInstall: false,
        featureFlags: {
          atlaspackV3: MODE === 'V3',
        },
      });

      await atlaspack[FUNCTION]();

      const buildTime = Date.now() - startTime;
      console.log(`Build ${i + 1}: ${buildTime}ms`);
      buildTimes.push(buildTime);
    }

    const average =
      buildTimes.reduce((total, time) => total + time, 0) / buildTimes.length;

    console.log(`Benchmarks completed with an average time of ${average}ms`);

    await writeJson(join(benchDir, 'report.json'), {
      average,
      buildTimes,
    });
  } catch (err) {
    console.error(err);
  } finally {
    await rmrf(tmpDir);

    // TEMP: AtlaspackV3 hangs when exiting
    process.exit(0);
  }
}

main();

function rmrf(target) {
  return rm(target, {force: true, recursive: true});
}

async function readJson(target) {
  return JSON.parse(await readFile(target, 'utf8'));
}

function writeJson(target, data) {
  return writeFile(target, JSON.stringify(data, null, 2));
}

function writeHeader(header) {
  if (headers > 1) {
    console.log('');
  }
  console.log(chalk.bold.cyan.underline(header.toUpperCase()));
  if (headers > 0) {
    console.log('');
  }
  headers += 1;
}
