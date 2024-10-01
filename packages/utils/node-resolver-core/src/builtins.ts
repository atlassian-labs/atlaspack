import {builtinModules} from 'module';
import nullthrows from 'nullthrows';
// flowlint-next-line untyped-import:off
// @ts-expect-error - TS2732 - Cannot find module '../package.json'. Consider using '--resolveJsonModule' to import module with '.json' extension.
import packageJson from '../package.json';

export const empty: string = require.resolve('./_empty.js');

let builtins: {
  [key: string]: {
    name: string;
    range: string | null | undefined;
  };
} =
  // $FlowFixMe
  Object.create(null);

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
  // @ts-expect-error - TS7053 - Element implicitly has an 'any' type because expression of type 'string' can't be used to index type '{ assert: string; buffer: string; console: string; constants: string; crypto: string; domain: string; events: string; http: string; https: string; os: string; path: string; process: string; punycode: string; ... 9 more ...; zlib: string; }'.
  let polyfill = polyfills[k];
  builtins[k] = {
    name: polyfill + (builtinModules.includes(polyfill) ? '/' : ''),
    range: nullthrows(packageJson.devDependencies[polyfill]),
  };
}

export default builtins;
