module.exports = {
  extends: '@atlaspack/eslint-config',
  parserOptions: {
    ecmaVersion: 5,
  },
  env: {
    browser: true,
  },
  rules: {
    'no-console': 'off',
    'no-global-assign': 'warn',
    'no-unused-vars': 'off',
  },
};
