const parcelBabelPreset = require('@atlaspack/babel-preset');
const path = require('path');

require('@babel/register')({
  cwd: path.join(__dirname, '../../..'),
  ignore: [
    (filepath) => filepath.includes(path.sep + 'node_modules' + path.sep),
    // Don't run babel over ignore integration tests fixtures.
    // These may include relative babel plugins, and running babel on those causes
    // the plugin to be loaded to compile the plugin.
    (filepath) =>
      filepath.endsWith('.js') &&
      filepath.includes('/core/integration-tests/test/integration'),
  ],
  only: [path.join(__dirname, '../../..')],
  presets: [parcelBabelPreset],
  plugins: [require('./babel-plugin-module-translate')],
  extensions: ['.js', '.jsx', '.ts', '.tsx'],
});

// This adds the registration to the Node args, which are passed
// to child processes by Node when we fork to create workers.
process.execArgv.push('-r', __filename);
