import {builtinModules} from 'module';
import nullthrows from 'nullthrows';
// flowlint-next-line untyped-import:off
import packageJson from '../package.json';

export const empty: string = require.resolve('./_empty.js');

let builtins: {
  [key: string]: {
    name: string;
    range: string | null | undefined;
  };
} = Object.create(null);

// use definite (current) list of Node builtins
for (let key of builtinModules) {
  builtins[key] = {name: empty, range: null};
}

let polyfills = {
  assert: 'assert',
  buffer: 'buffer',
  console: 'console-browserify',
  constants: 'constants-browserify',
  crypto: 'crypto-browserify',
  domain: 'domain-browser',
  events: 'events',
  http: 'stream-http',
  https: 'https-browserify',
  inspector: 'node-inspect-extracted',
  os: 'os-browserify',
  path: 'path-browserify',
  process: 'process',
  punycode: 'punycode',
  querystring: 'querystring-es3',
  stream: 'stream-browserify',
  string_decoder: 'string_decoder',
  sys: 'util',
  timers: 'timers-browserify',
  tty: 'tty-browserify',
  url: 'url',
  util: 'util',
  vm: 'vm-browserify',
  zlib: 'browserify-zlib',
};

for (let k in polyfills) {
  // @ts-expect-error TS7053
  let polyfill = polyfills[k];
  builtins[k] = {
    name: polyfill + (builtinModules.includes(polyfill) ? '/' : ''),
    // @ts-expect-error TS7053
    range: nullthrows(packageJson.devDependencies[polyfill]),
  };
}

export default builtins;
