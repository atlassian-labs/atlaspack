'use strict';

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
      code: "import logger from '../../../../core/logger';",
      errors: [
        {
          message: `Import for monorepo package '@atlaspack/logger' should be absolute.`,
        },
      ],
      filename,
      output: "import logger from '@atlaspack/logger/Logger';",
    },
  ],
});
