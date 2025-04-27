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
  'darwin-arm64': () => require('./artifacts/index.darwin-arm64.node'),
  'darwin-x64': () => require('./artifacts/index.darwin-x64.node'),
  'linux-arm-gnueabihf': () =>
    require('./artifacts/index.linux-arm-gnueabihf.node'),
  'linux-arm64-gnu': () => require('./artifacts/index.linux-arm64-gnu.node'),
  'linux-arm64-musl': () => require('./artifacts/index.linux-arm64-musl.node'),
  'linux-x64-gnu': () => require('./artifacts/index.linux-x64-gnu.node'),
  'linux-x64-musl': () => require('./artifacts/index.linux-x64-musl.node'),
  'win32-x64-msvc': () => require('./artifacts/index.win32-x64-msvc.node'),
};

module.exports = bindings[parts.join('-')]();
