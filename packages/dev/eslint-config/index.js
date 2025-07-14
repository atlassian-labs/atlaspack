module.exports = {
  extends: [
    'eslint:recommended',
    'plugin:flowtype/recommended',
    'plugin:monorepo/recommended',
    'plugin:react/recommended',
    'prettier',
  ],
  parser: '@babel/eslint-parser',
  plugins: [
    '@atlaspack/internal',
    'flowtype',
    'import',
    'monorepo',
    'react',
    'mocha',
  ],
  parserOptions: {
    ecmaVersion: 2018,
    ecmaFeatures: {
      jsx: true,
    },
    sourceType: 'module',
  },
  env: {
    es2020: true,
    node: true,
  },
  globals: {
    parcelRequire: true,
    define: true,
    SharedArrayBuffer: true,
  },
  // https://eslint.org/docs/user-guide/configuring#configuration-based-on-glob-patterns
  overrides: [
    {
      files: ['**/*.ts', '**/*.mts'],
      parser: '@typescript-eslint/parser',
      plugins: [
        '@atlaspack/internal',
        '@typescript-eslint',
        'import',
        'monorepo',
        'mocha',
      ],
      extends: [
        'eslint:recommended',
        'plugin:@typescript-eslint/recommended',
        'plugin:monorepo/recommended',
        'plugin:react/recommended',
        'prettier',
      ],
      rules: {
        // internal rules
        '@atlaspack/internal/no-self-package-imports': 'error',
        '@atlaspack/internal/no-ff-module-level-eval': 'error',
        '@atlaspack/internal/no-relative-import': 'error',
        'flowtype/no-types-missing-file-annotation': 'off',
        'prefer-const': 'off',
        // Temporary
        '@typescript-eslint/no-explicit-any': 'off',
        '@typescript-eslint/no-unused-vars': 'off',
        '@typescript-eslint/no-require-imports': 'off',
        '@typescript-eslint/no-misused-new': 'off',
        '@typescript-eslint/prefer-as-const': 'off',
        '@typescript-eslint/no-empty-object-type': 'off',
        '@typescript-eslint/no-unused-expressions': 'off',
        'no-prototype-builtins': 'off',
        'prefer-rest-params': 'off',
      },
    },
    {
      files: ['**/test/**', '*.test.js', 'packages/core/integration-tests/**'],
      env: {
        mocha: true,
      },
      rules: {
        'import/no-extraneous-dependencies': 'off',
        'monorepo/no-internal-import': 'off',
        '@atlaspack/internal/no-relative-import': 'off',
        'mocha/no-exclusive-tests': 'error',
      },
    },
  ],
  rules: {
    '@atlaspack/internal/no-self-package-imports': 'error',
    '@atlaspack/internal/no-ff-module-level-eval': 'error',
    'import/first': 'error',
    'import/newline-after-import': 'error',
    'import/no-extraneous-dependencies': 'error',
    'import/no-self-import': 'error',
    'no-prototype-builtins': 'off',
    'no-console': 'error',
    'no-return-await': 'error',
    'require-atomic-updates': 'off',
    'require-await': 'error',
    // Use internal rule
    'monorepo/no-relative-import': 'off',
    '@atlaspack/internal/no-relative-import': 'error',
  },
  settings: {
    flowtype: {
      onlyFilesWithFlowAnnotation: true,
    },
    react: {
      version: 'detect',
    },
  },
};
