const parcelBabelPreset = require('@atlaspack/babel-preset');
const path = require('path');
const fs = require('fs');

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
    // Include tests
    (filepath) =>
      filepath.endsWith('.js') &&
      !fs.readFileSync(filepath, 'utf8').trim().startsWith('// @flow'),
  ],
  only: [path.join(__dirname, '../../..')],
  presets: [parcelBabelPreset],
  plugins: [
    ...(process.env.ATLASPACK_REGISTER_USE_SRC === 'true'
      ? [require('./babel-plugin-module-translate')]
      : []),
  ],
  extensions: ['.js', '.jsx', '.ts', '.tsx'],
});

// This adds the registration to the Node args, which are passed
// to child processes by Node when we fork to create workers.
process.execArgv.push('-r', __filename);
