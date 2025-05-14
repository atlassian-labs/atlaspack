/* eslint-disable no-console */
import * as path from 'node:path';
import * as process from 'node:process';
import * as url from 'node:url';
import {$} from 'zx';
import glob from 'glob';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const __root = path.dirname(__dirname);

async function uploadDebugSymbolsToSentry() {
  $.stdio = 'inherit';

  const debugFiles = [];

  for (const foundRel of glob.sync('packages/**/*.node', {
    cwd: __root,
    ignore: '**/node_modules/**',
  })) {
    const found = path.join(__root, foundRel);
    console.log(`Stripping:     ${found}`);
    const output = `${found}.debug`;
    if (process.platform === 'linux') {
      await $`objcopy --only-keep-debug ${found} ${output}`;
      await $`objcopy --strip-debug --strip-unneeded ${found}`;
      await $`objcopy --add-gnu-debuglink=${output} ${found}`;
    } else if (process.platform === 'darwin') {
      const dsymOutput = `${found}.dsym`;
      debugFiles.push(dsymOutput);

      await $`dsymutil ${found} -o ${dsymOutput}`;
      await $`strip -x ${found} -o ${output}`;
    }
    debugFiles.push(output);

    console.log(`  ➜ Generated: ${output}`);
  }

  console.log('Uploading debug files to sentry');
  await $`yarn sentry-cli debug-files upload --include-sources --log-level=info .`;

  for (const debugFile of debugFiles) {
    await $`rm -rf ${debugFile}`;
    console.log('Deleted', debugFile);
  }
}

uploadDebugSymbolsToSentry();
