#!/usr/bin/env node

import {run} from './cli';

run().catch((err) => {
  // eslint-disable-next-line no-console
  console.error(err);
  process.exitCode = 1;
});
