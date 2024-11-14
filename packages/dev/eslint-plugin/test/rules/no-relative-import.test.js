'use strict';
// @flow

const {RuleTester} = require('eslint');
const rule = require('../../src/rules/no-relative-import');

const filename = __filename;

new RuleTester({
  parser: require.resolve('@babel/eslint-parser'),
  parserOptions: {ecmaVersion: 2018, sourceType: 'module'},
}).run('no-relative-import', rule, {
  valid: [{code: "import logger from '@atlaspack/logger';", filename}],
  invalid: [
    {
      code: `import Logger from '../../../../core/logger';`,
      errors: [
        {
          message: `Import for monorepo package '@atlaspack/logger' should be absolute.`,
        },
      ],
      filename,
      output: "import Logger from '@atlaspack/logger/lib/Logger';",
    },
    {
      code: `import type { PluginOptions } from '../../../../core/types-internal/src';`,
      errors: [
        {
          message: `Import for monorepo package '@atlaspack/types-internal' should be absolute.`,
        },
      ],
      filename,
      output: "import type { PluginOptions } from '@atlaspack/types-internal';",
    },
  ],
});
