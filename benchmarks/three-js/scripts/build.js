/* eslint-disable no-console */
/* eslint-disable import/no-extraneous-dependencies */

const path = require('node:path');
const fs = require('node:fs');
const os = require('node:os');
const {execSync: $} = require('node:child_process');
const {Atlaspack} = require('@atlaspack/core');

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

void (async function main() {
  console.log('Settings');
  console.log(`  Mode:        ${MODE}`);
  console.log(`  Plugins:     ${PLUGINS}`);
  console.log(`  Copies:      ${COPIES}`);

  // Atlaspack fails to build in the current directory because it is getting
  // settings from the workspace package.json. To get around this, this script
  // copies the benchmark to a temporary directory and links Atlaspack in
  let tmpDir;
  try {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'atlaspack-bench'));
  } catch (error) {
    console.error('Failed to create temp directory for benchmark', error);
    process.exit(1);
  }

  const paths = {
    '~': (...segments) => path.join(__dirname, '..', ...segments),
    '/tmp': (...segments) => path.join(tmpDir, ...segments),
  };

  try {
    console.log('Setup:');

    // Copy files to a temporary directory
    console.log('  Copying Base');

    rm(paths['~']('dist'));
    rm(paths['~']('.parcel-cache'));

    cp(paths['~'](), paths['/tmp']());
    rm(paths['/tmp']('node_modules'));

    // Patch the package.json to link the files to the workspace files
    const packageJson = readJson(paths['/tmp']('package.json'));
    for (const dependency of Object.keys(packageJson.dependencies)) {
      if (!dependency.startsWith('@atlaspack')) continue;
      const resolved = require.resolve(path.join(dependency, 'package.json'));
      const dir = path.dirname(resolved);
      packageJson.dependencies[dependency] = `file:${dir}`;
    }
    writeJson(paths['/tmp']('package.json'), packageJson);

    // Patch .parcelrc to include plugins
    const parcelRc = readJson(paths['/tmp']('.parcelrc'));
    for (let i = 0; i < PLUGINS; i++) {
      parcelRc['transformers']['*.{js,mjs,jsm,jsx,es6,cjs,ts,tsx}'].push(
        './plugins/transformer.js',
      );
    }
    writeJson(paths['/tmp']('.parcelrc'), parcelRc);

    // Get three-js
    if (fs.readdirSync(paths['~']('three-js')).length === 0) {
      console.log('  Pulling Three-js');
      $('git submodule update --init ./three-js', {
        cwd: paths['~'](),
        shell: true,
      });
    }

    // Copy three-js to bench directory
    console.log('  Copying Sources');

    for (let i = 0; i < COPIES; i++) {
      cp(paths['~']('three-js', 'src'), paths['/tmp']('src', `copy-${i}`));
      append(
        paths['/tmp']('src', 'index.js'),
        `import * as three_js_${i} from './copy-${i}/Three.js';`,
      );
      append(
        paths['/tmp']('src', 'index.js'),
        `globalThis['three_js_${i}'] = three_js_${i};\n`,
      );
    }

    // Link node_modules
    $('npm install', {
      cwd: paths['/tmp'](),
      shell: true,
    });

    // Start the benchmark
    console.log(`Running`);
    const startTime = Date.now();

    const atlaspack = new Atlaspack({
      shouldDisableCache: true,
      cacheDir: paths['/tmp']('.parcel-cache'),
      config: paths['/tmp']('.parcelrc'),
      entries: [paths['/tmp']('src', 'index.js')],
      targets: {
        default: {
          distDir: paths['/tmp']('dist'),
        },
      },
      shouldAutoInstall: false,
      featureFlags: {
        atlaspackV3: MODE === 'V3',
      },
    });

    await atlaspack.run();
    const buildTime = Date.now() - startTime;

    console.log(`  Build:       ${buildTime}ms`);

    writeJson(paths['~']('report.json'), {
      buildTime,
    });
  } catch (error) {
    console.error(error);
  } finally {
    rm(paths['/tmp']());

    // TEMP: AtlaspackV3 hangs when exiting
    process.exit(0);
  }
})();

function rm(target) {
  fs.rmSync(target, {force: true, recursive: true});
}

function cp(source, dest) {
  fs.cpSync(source, dest, {recursive: true});
}

function readJson(target) {
  return JSON.parse(fs.readFileSync(target, 'utf8'));
}
function writeJson(target, data) {
  fs.writeFileSync(target, JSON.stringify(data, null, 2));
}

function append(target, data) {
  fs.appendFileSync(target, data, 'utf8');
}
