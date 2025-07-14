#!/usr/bin/env node

'use strict';

const run = require('./cli').run;
require('v8-compile-cache');

run(process.argv.slice(2));
