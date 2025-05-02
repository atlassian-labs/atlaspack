/* eslint-disable */
let parts = [process.platform, process.arch];
if (process.platform === 'linux') {
  const {MUSL, family} = require('detect-libc');
  if (family === MUSL) {
    parts.push('musl');
  } else if (process.arch === 'arm') {
    parts.push('gnueabihf');
  } else {
    parts.push('gnu');
  }
} else if (process.platform === 'win32') {
  parts.push('msvc');
}

let bindings = {
  'darwin-x64': () => require('lightningcss-darwin-x64'),
  'linux-x64-gnu': () => require('lightningcss-linux-x64-gnu'),
  'win32-x64-msvc': () => require('lightningcss-win32-x64-msvc'),
  'win32-arm64-msvc': () => require('lightningcss-win32-arm64-msvc'),
  'darwin-arm64': () => require('lightningcss-darwin-arm64'),
  'linux-arm64-gnu': () => require('lightningcss-linux-arm64-gnu'),
  'linux-arm-gnueabihf': () => require('lightningcss-linux-arm-gnueabihf'),
  'linux-arm64-musl': () => require('lightningcss-linux-arm64-musl'),
  'linux-x64-musl': () => require('lightningcss-linux-x64-musl'),
  'freebsd-x64': () => require('lightningcss-freebsd-x64'),
};

try {
  module.exports = bindings[parts.join('-')]();
} catch (err) {
  module.exports = require(`../lightningcss.${parts.join('-')}.node`);
}

module.exports.browserslistToTargets = require('./browserslistToTargets');
module.exports.composeVisitors = require('./composeVisitors');
module.exports.Features = require('./flags').Features;
