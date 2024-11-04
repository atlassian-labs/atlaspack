/* eslint-disable no-console */
/* eslint-disable import/no-extraneous-dependencies */

const path = require('node:path');
const fs = require('node:fs');
const os = require('node:os');
const child_process = require('node:child_process');
const {Atlaspack} = require('@atlaspack/core');

const MODE = process.env.ATLASPACK_BENCH_MODE;
if (!MODE) {
  console.error('env:ATLASPACK_BENCH_MODE not specified');
  process.exit(1);
}
const USE_PLUGINS = process.env.ATLASPACK_BENCH_USE_PLUGINS;
if (!USE_PLUGINS) {
  console.error('env:ATLASPACK_BENCH_USE_PLUGINS not specified');
  process.exit(1);
}
const COPIES = process.env.ATLASPACK_BENCH_COPIES
  ? parseInt(process.env.ATLASPACK_BENCH_COPIES, 10)
  : 30;

void (async function main() {
  console.log('Settings');
  console.log(`  Mode:        ${MODE}`);
  console.log(`  Use Plugins: ${USE_PLUGINS}`);
  console.log(`  Copies:      ${COPIES}`);

  // Atlaspack fails to build in the current directory because it is getting
  // settings from the workspace package.json. To get around this, this script
  // copies the benchmark to a temporary directory and links Atlaspack in
  let tmpDir;
  try {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'atlaspack-bench'));
    // the rest of your app goes here
  } catch (error) {
    console.error('Failed to create temp directory for benchmark', error);
    process.exit(1);
  }

  try {
    // Copy files to a temporary directory
    fs.rmSync(path.join(__dirname, '..', 'dist'), {
      recursive: true,
      force: true,
    });
    fs.rmSync(path.join(__dirname, '..', '.parcel-cache'), {
      recursive: true,
      force: true,
    });

    fs.cpSync(path.join(__dirname, '..'), tmpDir, {recursive: true});
    fs.rmSync(path.join(tmpDir, 'node_modules'), {
      recursive: true,
      force: true,
    });

    // Patch the package.json to link the files to the workspace files
    const packageJson = JSON.parse(
      fs.readFileSync(path.join(tmpDir, 'package.json'), 'utf8'),
    );
    const newPackageJson = structuredClone(packageJson);
    for (const dependency of Object.keys(packageJson.dependencies)) {
      if (!dependency.startsWith('@atlaspack')) continue;
      newPackageJson.dependencies[dependency] = `file:${path.dirname(
        require.resolve(path.join(dependency, 'package.json')),
      )}`;
    }
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify(newPackageJson, null, 2),
    );

    // Link node_modules
    child_process.execSync('npm install', {cwd: tmpDir, shell: true});

    // Set up benchmark fixture
    child_process.execSync('tar -xzvf ./vendor/three-js.tar.gz -C ./vendor', {
      cwd: tmpDir,
      shell: true,
    });

    for (let i = 0; i < COPIES; i++) {
      fs.cpSync(
        path.join(tmpDir, 'vendor', 'three-js'),
        path.join(tmpDir, 'src', `copy-${i}`),
        {recursive: true},
      );
      fs.appendFileSync(
        path.join(tmpDir, 'src', 'index.js'),
        `import * as three_js_${i} from './copy-${i}/Three.js';`,
        'utf8',
      );
      fs.appendFileSync(
        path.join(tmpDir, 'src', 'index.js'),
        `globalThis['three_js_${i}'] = three_js_${i};\n`,
        'utf8',
      );
    }

    // Start the benchmark
    console.log(`Running`);
    const startTime = Date.now();

    const atlaspack = new Atlaspack({
      // inputFS: new NodeFS(),
      shouldDisableCache: true,
      cacheDir: path.join(tmpDir, '.parcel-cache'),
      config:
        USE_PLUGINS === 'true'
          ? path.join(tmpDir, '.parcelrc_plugins')
          : path.join(tmpDir, '.parcelrc'),
      entries: [path.join(tmpDir, 'src', 'index.js')],
      targets: {
        default: {
          distDir: path.join(tmpDir, 'dist'),
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

    fs.writeFileSync(
      path.join(__dirname, '..', 'report.json'),
      JSON.stringify({
        buildTime,
      }),
      'utf8',
    );
  } catch (error) {
    console.error(error);
  } finally {
    fs.rmSync(tmpDir, {recursive: true, force: true});

    // TEMP: AtlaspackV3 hangs when exiting
    process.exit(0);
  }
})();
