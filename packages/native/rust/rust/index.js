const process = require('node:process');

const arch = {
  x64: 'amd64',
  arm64: 'arm64',
}[process.arch];

const platform = {
  linux: 'linux',
  darwin: 'macos',
  win32: 'windows',
}[process.platform];

module.exports = require(`@atlaspack/rust-${platform}-${arch}`);
