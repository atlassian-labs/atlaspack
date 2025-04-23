let path = require('path');
let fs = require('fs/promises');
let findUp = require('find-up');

let lib_node_modules_dir = path.join(__dirname, '../lib/node_modules');

let bindings = [
  '@atlaspack/rust',
  '@parcel/source-map',
  'lightningcss',
  '@swc/core',
  // Uses dynamic requires internally so it can't easily be compiled
  'htmlnano',
];

async function main() {
  for (let package of bindings) {
    let packageDir = path.dirname(
      await findUp('package.json', {
        cwd: require.resolve(package),
      }),
    );
    let target = path.join(lib_node_modules_dir, package);

    await fs.cp(packageDir, target, {recursive: true});
  }
}

main();
