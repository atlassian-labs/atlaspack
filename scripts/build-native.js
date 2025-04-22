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

const cargoCommand = ['cargo', 'build', '--target', rustTarget];

if (CARGO_PROFILE && CARGO_PROFILE !== 'debug') {
  cargoCommand.push('--profile', CARGO_PROFILE);
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

      if (CARGO_PROFILE && CARGO_PROFILE !== 'debug') {
        command.push('--profile', CARGO_PROFILE);
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
  }
}
