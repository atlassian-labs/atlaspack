/* eslint-disable no-console */

/*
  This script takes an ESM TypeScript project and transpiles it
  to a valid ESM and CJS Nodejs output.

  The reason both formats are targetted is:

  1) Native Nodejs TypeScript support only strips types, so to have
     ESM code, the code must be valid ESM to begin with

  2) Backwards compatibility

  Eventually CJS outputs can be removed
*/

import * as fs from 'node:fs';
import * as url from 'node:url';
import * as path from 'node:path';
import {execFileSync} from 'node:child_process';
import glob from 'glob';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const __root = path.dirname(__dirname);

const bins = {
  tsc: path.join(__root, 'node_modules', '.bin', 'tsc'),
  swc: path.join(__root, 'node_modules', '.bin', 'swc'),
};

const LINK_LIB = process.env.LINK_LIB;

let [, , input = './src', output = './lib'] = process.argv;

if (!input || !output) {
  console.error('No input or output provided');
  console.error('  USAGE: node ./build-tsc.mjs ./src ./lib');
  process.exit(1);
}

if (!path.isAbsolute(input)) {
  input = path.join(process.cwd(), input);
}

if (!path.isAbsolute(output)) {
  output = path.join(process.cwd(), output);
}

const packageJsonPath = path.join(process.cwd(), 'package.json');
const pkg = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));

if (fs.existsSync(output)) {
  console.log('Resetting ./lib');
  execFileSync('git', ['clean', '-xdf', output], {
    cwd: __root,
    shell: true,
    stdio: 'inherit',
  });
}
fs.mkdirSync(output, {recursive: true});

console.log('Building TypeScript');
execFileSync('node', [bins.tsc], {
  cwd: process.cwd(),
  stdio: 'inherit',
  shell: true,
});

// Maintaining a CJS build is only required when there
// are consumers on Node.js <= 20.15.1
// Versions above that automatically load ESM (unless there are top level awaits)
console.log('Building Commonjs');

// Required to force .js files emitted to be
// interpretted as CommonJS files
fs.writeFileSync(
  path.join(output, 'package.json'),
  '{ "type": "commonjs" }',
  'utf8',
);

// For the SWC plugin cache, not important
const pluginCacheDir = path.join(
  __root,
  'node_modules',
  '.cache',
  'swc',
  pkg.name,
);

if (fs.existsSync(pluginCacheDir)) {
  fs.rmSync(pluginCacheDir, {recursive: true, force: true});
}

execFileSync(
  'node',
  [
    bins.swc,
    ...['-d', '.'],
    ...['--out-file-extension', 'js'],
    ...['--config-file', path.join(__dirname, 'build-tsc.swcrc.json')],
    ...['-C', `jsc.experimental.cacheRoot=${pluginCacheDir}`],
    ...glob.sync('**/*.mjs', {cwd: output}).map((v) => `./lib/${v}`),
  ],
  {
    cwd: process.cwd(),
    stdio: 'inherit',
    shell: true,
  },
);
