#!/usr/bin/env node

if (
  process.env.ATLASPACK_SOURCES === 'true' ||
  process.env.ATLASPACK_BUILD_ENV === 'test' ||
  process.env.ATLASPACK_SELF_BUILD
) {
  require('@atlaspack/babel-register');
}

const {main} = require('../src/cli');

main();
