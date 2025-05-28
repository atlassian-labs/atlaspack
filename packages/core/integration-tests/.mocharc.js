const mochaRequire = [
  '@atlaspack/babel-register',
  '@atlaspack/test-utils/src/mochaSetup.js',
];

if (process.env.ATLASPACK_PROFILE_MOCHA === 'true') {
  mochaRequire.push('@atlaspack/mocha-profiler');
}

module.exports = {
  extension: ['js', 'mjs', 'cjs', 'ts', 'cts', 'mts'],
  require: mochaRequire,
  timeout: 50000,
  _todo:
    'Remove exit: true when https://github.com/nodejs/node/pull/28788 is resolved',
  exit: true,
};
