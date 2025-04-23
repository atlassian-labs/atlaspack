#!/usr/bin/env node

// tsx can be removed once we move to Node LTS which
// has built in TypeScript support

import process from 'node:process';
import fs from 'node:fs';
import path from 'node:path';
import url from 'node:url';
import * as tsxEsm from 'tsx/esm/api';
import * as tsxCjs from 'tsx/cjs/api';

const __filename = url.fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

const cjs = tsxCjs.register({
  namespace: Date.now().toString()
})

const esm = tsxEsm.register({
  namespace: Date.now().toString()
})

const [command] = process.argv.splice(2, 1);
const commandPath = path.join(__dirname, command);

if (fs.existsSync(`${commandPath}.js`)) {
  require(`${commandPath}.js`);
} else if (fs.existsSync(`${commandPath}.cjs`)) {
  require(`${commandPath}.cjs`);
} else if (fs.existsSync(`${commandPath}.mjs`)) {
  import(`${commandPath}.mjs`);
} else if (fs.existsSync(`${commandPath}.ts`)) {
  cjs.require(`${commandPath}.ts`, __filename);
} else if (fs.existsSync(`${commandPath}.mts`)) {
  esm.import(`${commandPath}.mts`, import.meta.url);
} else if (fs.existsSync(`${commandPath}.cts`)) {
  cjs.require(`${commandPath}.cts`, __filename);
} else {
  // eslint-disable-next-line no-console
  console.error(`Command "${command}" not found`);
  process.exit(1);
}
