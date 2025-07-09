#!/usr/bin/env node

'use strict';

if (
  process.env.ATLASPACK_SOURCES === 'true' ||
  process.env.ATLASPACK_BUILD_ENV === 'test' ||
  process.env.ATLASPACK_SELF_BUILD
) {
  require('@atlaspack/babel-register');
}

const run = require('./cli').run;
require('v8-compile-cache');

run(process.argv.slice(2));
