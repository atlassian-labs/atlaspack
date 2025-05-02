let path = require('path');
let {Transformer} = require('@atlaspack/plugin');

const patches = {
  'node_modules/htmlnano/lib/htmlnano.js': '../patches/htmlnano.js',
  'packages/core/rust/index.js': '../patches/atlaspack-rust.js',
  'node_modules/@parcel/watcher/index.js': '../patches/parcel-watcher.js',
  'node_modules/@parcel/source-map/parcel_sourcemap_node/index.js':
    '../patches/parcel-source-map.js',
  'node_modules/lightningcss/node/index.js': '../patches/lightning.js',
  'packages/core/core/src/internal-plugins.js':
    '../patches/internal-plugins.js',
};

const rootDir = path.join(__dirname, '../../../..');

module.exports = new Transformer({
  async transform({asset, options}) {
    let patch = patches[path.relative(rootDir, asset.filePath)];

    if (patch) {
      let patchedCode = await options.inputFS.readFile(
        path.join(__dirname, patch),
      );
      asset.setCode(patchedCode);
    }

    return [asset];
  },
});
