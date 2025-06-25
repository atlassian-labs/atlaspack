#!/usr/bin/env node

'use strict';

if (
  process.env.ATLASPACK_DEV === 'true' ||
  process.env.ATLASPACK_BUILD_ENV === 'test' ||
  process.env.ATLASPACK_SELF_BUILD
) {
  require('@atlaspack/babel-register');
}

require('./cli');
