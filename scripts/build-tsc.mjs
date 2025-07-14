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
  prettier: path.join(__root, 'node_modules', '.bin', 'prettier'),
};

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

if (fs.existsSync(output)) {
  console.log('Resetting ./lib');
  execFileSync('git', ['clean', '-xdf', output], {
    cwd: __root,
    shell: true,
    stdio: 'inherit',
  });
}

console.log('Building TypeScript');
execFileSync('node', [bins.tsc], {
  cwd: process.cwd(),
  stdio: 'inherit',
  shell: true,
});

for (const entry of glob.sync('**/*.d.*', {cwd: output})) {
  const {name} = path.parse(entry);
  fs.renameSync(path.join(output, entry), path.join(output, `${name}.ts`));
}

console.log('Building Commonjs');
execFileSync(
  'node',
  [
    bins.swc,
    ...glob.sync('**/*.mjs', {cwd: output}).map((v) => `./lib/${v}`),
    ...['-d', '.'],
    ...['--out-file-extension', 'js'],
    ...['-C', 'module.type=commonjs'],
    ...['-C', 'isModule=true'],
    ...['-C', 'jsc.target=es2024'],
  ],
  {
    cwd: process.cwd(),
    stdio: 'inherit',
    shell: true,
  },
);

console.log('Tidy up');
execFileSync('node', [bins.prettier, './lib/**/*.js', './lib/**/*.mjs', '-w'], {
  cwd: process.cwd(),
  stdio: 'inherit',
  shell: true,
});
