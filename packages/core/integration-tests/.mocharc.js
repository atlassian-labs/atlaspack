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
  config.exit = true;
}

// V3 has open handles (ThreadSafeFunctions) that don't close properly after tests,
// causing mocha to hang. Force exit to work around this issue.
// See: packages/core/integration-tests/test/babel.ts afterEach for related workaround.
if (process.env.ATLASPACK_V3 === 'true') {
  config.exit = true;
}

module.exports = config;
