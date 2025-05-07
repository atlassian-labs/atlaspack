/* eslint-disable no-console */
const fs = require('fs');
const glob = require('glob');
const path = require('path');
const child_process = require('child_process');
const process = require('node:process');

const __root = path.normalize(path.join(__dirname, '..'));

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

const cargoCommand = ['cargo', 'build', '--target', rustTarget];

if (rustProfile !== 'dev') {
  cargoCommand.push('--profile', rustProfile);
}

// eslint-disable-next-line no-console
console.log(cargoCommand.join(' '));
child_process.execSync(cargoCommand.join(' '), {stdio: 'inherit', cwd: __root});

// Go through npm packages and run custom native commands on them
const {workspaces} = JSON.parse(
  fs.readFileSync(path.join(__root, 'package.json'), 'utf8'),
);

for (const workspace of workspaces) {
  for (const pkg of glob.sync(workspace, {cwd: __root})) {
    const pkgDir = path.join(__root, pkg);
    buildNapiLibrary(pkgDir);
    copyBinaries(pkgDir);
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
    process.exit(1);
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
    process.exit(1);
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
    process.exit(1);
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
