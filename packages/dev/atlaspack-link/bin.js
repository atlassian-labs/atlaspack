#! /usr/bin/env node

/* eslint-disable no-console */

'use strict';

require('@atlaspack/babel-register');

let program = require('./src/cli').createProgram();

(async function main() {
  try {
    await program.parseAsync();
  } catch (e) {
    console.error(e);
    process.exit(1);
  }
})();
