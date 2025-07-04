import importCondTypeAnnotationsRule from './rules/importcond-type-annotations';
import noImportCondExportsRule from './rules/no-importcond-exports';

export const {name, version} =
  // eslint-disable-next-line @typescript-eslint/no-var-requires
  require('../package.json') as typeof import('../package.json');

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
