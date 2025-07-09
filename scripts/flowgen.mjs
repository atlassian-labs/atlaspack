/* eslint-disable no-console */
import * as fs from 'node:fs';
import * as path from 'node:path';
import * as flowgen from 'flowgen';

let [, , input, output] = process.argv;

if (!input || !output) {
  console.error('No input or output provided');
  console.error('  USAGE: node ./flowgen.mjs ./input.d.ts ./output.js');
  process.exit(1);
}

if (!path.isAbsolute(input)) {
  input = path.join(process.cwd(), input);
}

if (!path.isAbsolute(output)) {
  output = path.join(process.cwd(), output);
}

if (!fs.existsSync(input)) {
  console.error('input does not exist');
  process.exit(1);
}

const file = fs.readFileSync(input, 'utf8');
const flowdef = flowgen.compiler.compileDefinitionString(file);
const flowDefPretty = flowgen.beautify(flowdef);
const flowWithHeader = `// @flow\n\n${flowDefPretty}`;

fs.writeFileSync(output, flowWithHeader, 'utf8');
