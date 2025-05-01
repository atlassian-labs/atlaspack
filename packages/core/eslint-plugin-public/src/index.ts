import importCondTypeAnnotationsRule from './rules/importcond-type-annotations';
import noImportCondExportsRule from './rules/no-importcond-exports';

// eslint-disable-next-line @typescript-eslint/no-require-imports
const {name, version} =
  require('../package.json') as typeof import('../package.json');

const rules = {
  'importcond-type-annotations': importCondTypeAnnotationsRule,
  'no-importcond-exports': noImportCondExportsRule,
};

const plugin = {
  meta: {
    name,
    version,
  },
  rules,
  configs: {
    get recommended() {
      return recommended;
    },
  },
} as const;

const recommended = {
  plugins: {
    '@atlaspack': plugin,
  },
  rules: {
    '@atlaspack/importcond-type-annotations': 'error',
    '@atlaspack/no-importcond-exports': 'error',
  },
};

export default plugin;
