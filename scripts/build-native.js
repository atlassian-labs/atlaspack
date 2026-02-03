/* eslint-disable no-console */
const fs = require('fs');
const glob = require('glob');
const path = require('path');
const child_process = require('child_process');
const process = require('node:process');

const __root = path.normalize(path.join(__dirname, '..'));

// Parse command line arguments
const args = process.argv.slice(2);
const cleanNapi = args.includes('--clean-napi');
const fastMode = args.includes('--fast');

// Do a full cargo build
const CARGO_PROFILE = process.env.CARGO_PROFILE;
const RUSTUP_TARGET = process.env.RUST_TARGET || process.env.RUSTUP_TARGET;

const defaultTarget = {
  'linux-x64': 'x86_64-unknown-linux-gnu',
  'linux-arm64': 'aarch64-unknown-linux-gnu',
  'darwin-x64': 'x86_64-apple-darwin',
  'darwin-arm64': 'aarch64-apple-darwin',
  'win32-x64': 'x86_64-pc-windows-msvc',
  'win32-arm64': 'aarch64-pc-windows-msvc',
}[`${process.platform}-${process.arch}`];

const rustTarget = RUSTUP_TARGET || defaultTarget;
let rustProfile = CARGO_PROFILE || 'dev';

// If --clean-napi flag is set, clean all crates with #[napi] annotations before building
if (cleanNapi) {
  cleanNapiCrates();
}
if (!fastMode) {
  // Only build APVM with `cargo build` - this prevents building unnecessary dependencies
  // when re-building for NAPI as the environment is different and it triggers build.rs files to run
  const cargoCommand = ['cargo', 'build', '--target', rustTarget, '-p apvm'];

  if (rustProfile !== 'dev') {
    cargoCommand.push('--profile', rustProfile);
  }

  // eslint-disable-next-line no-console
  console.log(cargoCommand.join(' '));
  child_process.execSync(cargoCommand.join(' '), {
    stdio: 'inherit',
    cwd: __root,
  });
}

// Go through npm packages and run custom native commands on them
const {workspaces} = JSON.parse(
  fs.readFileSync(path.join(__root, 'package.json'), 'utf8'),
);

for (const workspace of workspaces) {
  for (const pkg of glob.sync(workspace, {cwd: __root})) {
    const pkgDir = path.join(__root, pkg);
    buildNapiLibrary(pkgDir);
    if (!fastMode) {
      copyBinaries(pkgDir);
    }
  }
}

/*
  This script enables workspace support for @napi-rs/cli.

  napi-rs expects there to be one package and no workspace, the "napi build"
  command will trigger a "cargo build" which generates the
  respective dynamic libraries in

  `~/target/<target>/<profile>/libname.(so|dylib|dll)

  napi-rs then reads the cargo build manifest and copies the dynamic library

  from ~/target/../libname.(so|dylib|dll)
  to   ~/packages/package-name/libname.node

  then generate TypeScript types and an index.js file

  If you have already run "cargo build" before running "napi build",
  "napi build" will reuse the existing artifacts (it will still run
  a "cargo build" but the output will be cached/instant).

  To make this work in workspaces, this script will search through the workspace
  packages, where packages that contain a "package.json@napi" key will have
  "napi build" run for them with the correct cwd specified.

  USAGE:

  To include napi artifacts in a package the package needs to define the following
  in the package.json

  ```json
  {
    "napi": {
      // "name" in the Cargo.toml
      "name": "cargo_package_name",

      // This key defines cargo targets that
      // are permitted to be copied into the package.
      // This is used for packages dedicated to an os/arch
      "permittedTargets": [
        "x86_64-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        "*"                           // can specify any
                                      // target with wildcard
      ],

      // Optional: if the package only exports one .node file
      // this will overwrite the index file to simplify it so
      // it only reexports the one .node file
      "overwriteIndex": false

      // Optional: skip build for package
      "skip": false
    }
  }
  ```
*/
function buildNapiLibrary(pkgDir) {
  let pkgJsonPath = path.join(pkgDir, 'package.json');
  if (!fs.existsSync(pkgJsonPath)) return;

  const pkgJson = JSON.parse(fs.readFileSync(pkgJsonPath, 'utf8'));

  // NAPI: Generate types and copy binaries for napi packages
  if (pkgJson.napi && pkgJson.napi.skip) return;
  if (!arrayOrStringHas(pkgJson.napi?.permittedTargets, rustTarget)) return;

  if (rustTarget === 'wasm32-unknown-unknown') {
    // "wasm:build":
    //   cargo build -p atlaspack-node-bindings --target wasm32-unknown-unknown
    //   cp ../../../target/wasm32-unknown-unknown/debug/atlaspack_node_bindings.wasm .
    // "wasm:build-release":
    //    "CARGO_PROFILE_RELEASE_LTO=true cargo build -p atlaspack-node-bindings --target wasm32-unknown-unknown --release
    //    wasm-opt --strip-debug -O ../../../target/wasm32-unknown-unknown/release/atlaspack_node_bindings.wasm -o atlaspack_node_bindings.wasm"
    console.error('Not supported');
    process.exitCode = 1;
    return;
  }

  const command = [];

  command.push(
    'npx',
    'napi',
    'build',
    '--platform', // Tell napi to build types and index.js
    '--target',
    rustTarget,
  );

  if (rustProfile !== 'dev') {
    command.push('--profile', rustProfile);
  }

  command.push('--cargo-name', pkgJson.napi.name.replaceAll('-', '_'));
  command.push(path.relative(__root, pkgDir));

  console.log(pkgJson.name, command.join(' '));

  // npx napi build must be run from the workspace root
  // to avoid cargo doing a complete build

  // Reference
  // npx napi build --platform --target {} --profile {} --cargo-name {Cargo.toml#name} {./path/to/package}
  child_process.execSync(command.join(' '), {
    stdio: 'inherit',
    cwd: __root,
  });

  // If the package is exporting a single .node binary then
  // overwrite the index.js to export only that binary
  // This is for @atlaspack/pkg-{platform}-{arch} packages
  if (pkgJson.napi.overwriteIndex) {
    for (const file of fs.readdirSync(pkgDir)) {
      if (!file.endsWith('.node')) {
        continue;
      }
      fs.writeFileSync(
        path.join(pkgDir, 'index.js'),
        `module.exports = require('./${file}');\n`,
        'utf8',
      );
      break;
    }
  }
}

/*
  This script will automatically copy binaries from target into a package
  if that package requests native binaries

  USAGE:

  ```json
  //package.json
  {
    "copyBin": {
      // Name of executable under ~/target/<target>/<profile>/<exename>
      "name": "exename",

      // Optional: rename the executable to the specified name
      "dest": "exename-renamed",

      // This key defines cargo targets that
      // are permitted to be copied into the package.
      // This is used for packages dedicated to an os/arch
      "permittedTargets": [
        "x86_64-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        "*"                           // can specify any
                                      // target with wildcard
      ],
    }
  }
  ```
*/
function copyBinaries(pkgDir) {
  let pkgJsonPath = path.join(pkgDir, 'package.json');
  if (!fs.existsSync(pkgJsonPath)) return false;
  const pkgJson = JSON.parse(fs.readFileSync(pkgJsonPath, 'utf8'));

  if (!arrayOrStringHas(pkgJson.copyBin?.permittedTargets, rustTarget)) return;
  if (!pkgJson.copyBin.name) {
    console.error('No bin specified for', pkgJson.name);
    process.exitCode = 1;
    return;
  }

  const sourceBin = path.join(
    'target',
    rustTarget,
    rustProfile === 'dev' ? 'debug' : rustProfile,
    pkgJson.copyBin.name,
  );

  const targetBin = path.join(
    pkgDir,
    pkgJson.copyBin.dest || pkgJson.copyBin.name,
  );

  if (!fs.existsSync(sourceBin)) {
    console.error('Binary not found', pkgJson.name, pkgJson.copyBin.name);
    process.exitCode = 1;
    return;
  }

  if (fs.existsSync(targetBin)) {
    fs.rmSync(targetBin);
  }

  fs.cpSync(sourceBin, targetBin);
}

function arrayOrStringHas(target, contains) {
  if (!target) {
    return false;
  }
  if (
    Array.isArray(target) &&
    (target.includes(contains) || target.includes('*'))
  ) {
    return true;
  }
  if (typeof target === 'string' && (target === contains || target === '*')) {
    return true;
  }
  return false;
}

/**
 * Find all local crates that contain #[napi] annotations and clean them.
 * This forces a rebuild of those crates, which will regenerate the intermediate
 * type files and fix stale type definition issues.
 */
function cleanNapiCrates() {
  console.log('Cleaning crates with #[napi] annotations...');

  try {
    // Use ripgrep to find all Rust files with #[napi] annotations
    // Handle both ripgrep not being available and no matches found
    let rgOutput = '';
    try {
      rgOutput = child_process.execSync(
        'rg -l "#\\[napi\\]" crates/ packages/ --type rust 2>/dev/null || true',
        {cwd: __root, encoding: 'utf8'},
      );
    } catch (e) {
      // Check if ripgrep is not available (ENOENT) vs no matches (other errors)
      if (e.code === 'ENOENT') {
        console.error(
          'Error: ripgrep (rg) is required for --clean-napi. Please install it.',
        );
        process.exitCode = 1;
        return;
      }
      // ripgrep returns non-zero if no matches found, which is fine
      rgOutput = '';
    }

    const filesWithNapi = rgOutput
      .split('\n')
      .filter((line) => line.trim().length > 0);

    if (filesWithNapi.length === 0) {
      console.log('No crates with #[napi] annotations found.');
      return;
    }

    // Extract unique crate directories from file paths
    const crateDirs = new Set();
    for (const file of filesWithNapi) {
      // Find the Cargo.toml directory (go up from src/ or lib.rs location)
      const filePath = path.join(__root, file);
      let dir = path.dirname(filePath);

      // Walk up to find Cargo.toml
      while (dir !== __root && dir !== path.dirname(__root)) {
        const cargoToml = path.join(dir, 'Cargo.toml');
        if (fs.existsSync(cargoToml)) {
          crateDirs.add(dir);
          break;
        }
        dir = path.dirname(dir);
      }
    }

    // Read Cargo.toml for each crate to get the package name
    const crateNames = [];
    for (const crateDir of crateDirs) {
      const cargoTomlPath = path.join(crateDir, 'Cargo.toml');
      try {
        const cargoToml = fs.readFileSync(cargoTomlPath, 'utf8');
        const nameMatch = cargoToml.match(/^name\s*=\s*["']([^"']+)["']/m);
        if (nameMatch) {
          crateNames.push(nameMatch[1]);
        }
      } catch (e) {
        console.warn(`Failed to read ${cargoTomlPath}:`, e.message);
      }
    }

    if (crateNames.length === 0) {
      console.log('No crate names found to clean.');
      return;
    }

    console.log(
      `Found ${crateNames.length} crate(s) with #[napi] annotations:`,
    );
    for (const name of crateNames) {
      console.log(`  - ${name}`);
    }

    // Clean each crate
    for (const crateName of crateNames) {
      console.log(`Cleaning crate: ${crateName}`);
      const cleanCommand = [
        'cargo',
        'clean',
        '-p',
        crateName,
        '--target',
        rustTarget,
      ];
      try {
        child_process.execSync(cleanCommand.join(' '), {
          stdio: 'inherit',
          cwd: __root,
        });
      } catch (e) {
        console.warn(`Failed to clean ${crateName}:`, e.message);
      }
    }

    console.log('Finished cleaning napi crates.');
  } catch (e) {
    console.error('Error during napi crate cleaning:', e.message);
    // Don't fail the build if cleaning fails
  }
}
