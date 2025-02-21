import * as fs from 'node:fs';
import {Paths} from './paths.ts';

// Rewrite @atlaspack/* in output to use package.json#imports
export const packageImports = JSON.parse(
  fs.readFileSync(Paths['~/']('package.json'), 'utf8'),
).imports;

export const localResolutions = {};
for (const key in packageImports) {
  localResolutions[key.replace('#', '@')] = key;
}

export const packageMappings = [
  ['@atlaspack/bundler-', '#atlaspack/bundler/'],
  ['@atlaspack/compressor-', '#atlaspack/compressor/'],
  ['@atlaspack/config-', '#atlaspack/config/'],
  ['@atlaspack/namer-', '#atlaspack/namer/'],
  ['@atlaspack/optimizer-', '#atlaspack/optimizer/'],
  ['@atlaspack/packager-', '#atlaspack/packager/'],
  ['@atlaspack/reporter-', '#atlaspack/reporter/'],
  ['@atlaspack/resolver-', '#atlaspack/resolver/'],
  ['@atlaspack/runtime-', '#atlaspack/runtime/'],
  ['@atlaspack/transformer-', '#atlaspack/transformer/'],
  ['@atlaspack/validator-', '#atlaspack/validator/'],
];
