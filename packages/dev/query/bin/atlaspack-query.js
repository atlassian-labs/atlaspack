#!/usr/bin/env node
require('../lib/bin');

if (
  process.env.ATLASPACK_REGISTER_USE_SRC === 'true' ||
  process.env.ATLASPACK_BUILD_ENV === 'test' ||
  process.env.ATLASPACK_SELF_BUILD
) {
  require('@atlaspack/babel-register');
  require('../src/bin');
} else {
  require('../lib/bin');
}
