const {createDefaultPreset} = require('ts-jest');

const tsJestTransformCfg = createDefaultPreset().transform;

/** @type {import("jest").Config} **/
module.exports = {
  testEnvironment: 'jsdom',
  transform: {
    ...tsJestTransformCfg,
  },
  moduleNameMapper: {
    '\\.css$': '<rootDir>/src/test/stubCssModule.js',
    '\\.module\\.css$': '<rootDir>/src/test/stubCssModule.js',
  },
  collectCoverageFrom: ['src/**/*.tsx'],
};
