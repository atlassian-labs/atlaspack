const isCI = process.env.CI === 'true';
const isSuperPackageBuild = process.env.SUPER_PACKAGE === 'true';

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
  // 'Remove exit: true when https://github.com/nodejs/node/pull/28788 is resolved'
  exit: true,
  retries: isCI ? 2 : 0,
  ignore: isSuperPackageBuild
    ? // Ignore irrelevant tests for the super package
      [
        'test/atlaspack-link.js',
        'test/atlaspack-query.js',
        'test/cache.js',
        'test/incremental-bundling.js',
      ]
    : [],
};

module.exports = config;
