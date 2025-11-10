/* eslint-disable no-console */
import {$, cd} from 'zx';
import {join} from 'node:path';
import {existsSync} from 'node:fs';

const PROMPT = `
We are building a Compiled CSS-in-JS swc plugin to replace the babel plugin '@compiled/babel-plugin'  
  
We need to fix all the differences between the babel plugin and the swc plugin.
Fix the differences by doing the following continuously:

- Run node scripts/compare-compiled-css.js and observe the differences in comparison-results/
- If the SWC plugin is different, update it to match the output of the babel plugin by changing crates/atlassian-swc-compiled-css
- Run node scripts/compare-compiled-css.js again to verify the differences are fixed. This will also rebuild the project.
- Continue until all differences are fixed
`;

async function main() {
  console.log('🚀 Starting rovodev script...');

  let iteration = 0;
  while (true) {
    iteration++;
    console.log(`\n📍 === Iteration ${iteration} ===`);

    // Create the process and pipe output to stdout/stderr in real-time
    const proc = $`acli rovodev run --yolo "${PROMPT}"`.nothrow();
    proc.stdout.pipe(process.stdout);
    proc.stderr.pipe(process.stderr);

    try {
      await proc;
    } catch (error) {
      console.error('⚠️  Command exited with status:', error.exitCode);
    }
  }
}

await main();
