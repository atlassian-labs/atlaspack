/**
 * DEV BIN - DO NOT PUBLISH
 *
 * This file is copied into /lib/ by `yarn run dev:prepare`
 *
 * When babel build runs it is overwritten by another asset.
 */
process.env.ATLASPACK_REGISTER_USE_SRC = 'true';
require('@atlaspack/babel-register');
require('../src/bin');
