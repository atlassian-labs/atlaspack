let path = require('path');
let fs = require('fs/promises');
let glob = require('fast-glob');
let findUp = require('find-up');

let lib_node_modules_dir = path.join(__dirname, '../lib/node_modules');

let bindings = [
  '@atlaspack/rust',
  '@parcel/source-map',
  // The following bindings use optional deps for native bindings
  'lightningcss*',
  '@swc/core*',
  // Uses dynamic requires internally so it can't easily be compiled
  'htmlnano',
];

async function main() {
  let packageJsons = await glob(
    bindings.map((binding) => `node_modules/${binding}/package.json`),
    {
      cwd: path.join(__dirname, '../../../..'),
      absolute: true,
    },
  );

  for (let packageJson of packageJsons) {
    let packageName = require(packageJson).name;
    let packageDir = await fs.realpath(path.dirname(packageJson));
    let target = path.join(lib_node_modules_dir, packageName);

    await fs.cp(packageDir, target, {recursive: true});
  }
}

main();
