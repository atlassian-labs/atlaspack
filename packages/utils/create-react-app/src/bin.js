#!/usr/bin/env node

'use strict';

if (
  process.env.ATLASPACK_DEV === 'true' ||
  process.env.ATLASPACK_BUILD_ENV === 'test'
) {
  require('@atlaspack/babel-register');
}

require('v8-compile-cache');
require('./cli');
