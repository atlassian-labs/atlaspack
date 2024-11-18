'use strict';

module.exports = {
  rules: {
    'no-self-package-imports': require('./src/rules/no-self-package-imports'),
    'no-ff-module-level-eval': require('./src/rules/no-ff-module-level-eval'),
    'no-relative-import': require('./src/rules/no-relative-import'),
  },
};
