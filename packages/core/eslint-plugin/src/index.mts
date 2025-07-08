import * as fs from 'node:fs';
import * as path from 'node:path';
import * as url from 'node:url';
import importCondTypeAnnotationsRule from './rules/importcond-type-annotations/index.mts';
import noImportCondExportsRule from './rules/no-importcond-exports/index.mts';

const dirname = path.dirname(url.fileURLToPath(import.meta.url));

export const {name, version} = JSON.parse(
  fs.readFileSync(path.join(dirname, '..', 'package.json'), 'utf8'),
);

export const rules = {
  'importcond-type-annotations': importCondTypeAnnotationsRule,
  'no-importcond-exports': noImportCondExportsRule,
};

const recommended = {
  plugins: ['@atlaspack'],
  rules: {
    '@atlaspack/importcond-type-annotations': 'error',
    '@atlaspack/no-importcond-exports': 'error',
  },
} as const;

export const plugin = {
  meta: {
    name,
    version,
  },
  rules,
  configs: {
    recommended,
    get 'flat/recommended'() {
      return flatRecommended;
    },
  },
} as const;

const flatRecommended = {
  plugins: {
    '@atlaspack': plugin,
  },
  rules: {
    '@atlaspack/importcond-type-annotations': 'error',
    '@atlaspack/no-importcond-exports': 'error',
  },
} as const;

export const configs = plugin.configs;

export default plugin;
