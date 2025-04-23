/* eslint-disable no-console */
const fs = require('fs');
const glob = require('glob');
const path = require('path');
const child_process = require('child_process');
const process = require('node:process');

const __root = path.normalize(path.join(__dirname, '..'));

// Do a full cargo build
const CARGO_PROFILE = process.env.CARGO_PROFILE;
const RUSTUP_TARGET = process.env.RUSTUP_TARGET;

const defaultTarget = {
  'linux-x64': 'x86_64-unknown-linux-gnu',
  'linux-arm64': 'aarch64-unknown-linux-gnu',
  'darwin-x64': 'x86_64-apple-darwin',
  'darwin-arm64': 'aarch64-apple-darwin',
  'win32-x64': 'x86_64-pc-windows-msvc',
  'win32-arm64': 'aarch64-pc-windows-msvc',
}[`${process.platform}-${process.arch}`];

const rustTarget = RUSTUP_TARGET || defaultTarget;
const rustProfile = CARGO_PROFILE || 'debug';

const cargoCommand = ['cargo', 'build', '--target', rustTarget];

if (rustProfile !== 'debug') {
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
  for (const pkg of glob.sync(`${workspace}/package.json`, {cwd: __root})) {
    let pkgPath = path.join(__root, pkg);
    let pkgDir = path.dirname(pkgPath);
    const pkgJson = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));

    // NAPI: Generate types and copy binaries for napi packages
    if (pkgJson.napi && !pkgJson.napi.skip) {
      const command = [];

      if (rustTarget === 'wasm32-unknown-unknown') {
        // Not supported
        // "wasm:build": "cargo build -p atlaspack-node-bindings --target wasm32-unknown-unknown && cp ../../../target/wasm32-unknown-unknown/debug/atlaspack_node_bindings.wasm .",
        // "wasm:build-release": "CARGO_PROFILE_RELEASE_LTO=true cargo build -p atlaspack-node-bindings --target wasm32-unknown-unknown --release && wasm-opt --strip-debug -O ../../../target/wasm32-unknown-unknown/release/atlaspack_node_bindings.wasm -o atlaspack_node_bindings.wasm"
      } else {
        command.push(
          'npx',
          'napi',
          'build',
          '--platform',
          '--target',
          rustTarget,
        );
      }

      if (rustProfile !== 'debug') {
        command.push('--profile', rustProfile);
      }

      command.push(
        '--cargo-cwd',
        path.relative(pkgDir, path.join(__root, 'crates', 'node-bindings')),
      );

      // eslint-disable-next-line no-console
      console.log(pkgJson.name, command.join(' '));
      child_process.execSync(command.join(' '), {
        stdio: 'inherit',
        cwd: pkgDir,
      });
    }

    // Copy binaries for packages that distribute bins
    if (pkgJson.copyBin && pkgJson.copyBin.rustTarget === rustTarget) {
      if (!pkgJson.copyBin.name) {
        console.error('No bin specified for', pkgJson.name);
        process.exit(1);
      }
      const sourceBin = path.join(
        'target',
        rustTarget,
        rustProfile,
        pkgJson.copyBin.name,
      );
      const targetBin = path.join(
        pkgDir,
        pkgJson.copyBin.dest || pkgJson.copyBin.name,
      );

      if (!fs.existsSync(sourceBin)) {
        console.error('No bin exists for', pkgJson.name, pkgJson.copyBin.name);
        process.exit(1);
      }

      if (fs.existsSync(targetBin)) {
        fs.rmSync(targetBin);
      }

      fs.cpSync(sourceBin, targetBin);
    }
  }
}
