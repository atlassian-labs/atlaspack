const mochaRequire = [
  '@atlaspack/babel-register',
  '@atlaspack/test-utils/src/mochaSetup.js',
];

if (process.env.ATLASPACK_PROFILE_MOCHA === 'true') {
  mochaRequire.push('@atlaspack/mocha-profiler');
}

const config = {
  extension: ['js', 'mjs', 'cjs', 'ts', 'cts', 'mts'],
  require: mochaRequire,
  timeout: 50000,
};

if (process.env.ATLASPACK_INTEGRATION_TESTS_CI === 'true') {
  config.retries = 2;
  config.timeout = 50000;
}

module.exports = config;
