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

    if (process.platform === 'linux') {
      const output = `${found}.debug`;

      await $`objcopy --only-keep-debug --compress-debug-sections=zlib ${found} ${output}`;
      await $`objcopy --strip-debug --strip-unneeded ${found}`;
      await $`objcopy --add-gnu-debuglink=${output} ${found}`;

      debugFiles.push(output);
    } else if (process.platform === 'darwin') {
      const dsymOutput = `${found}.dsym`;

      await $`dsymutil ${found} -o ${dsymOutput}`;
      await $`strip -x ${found}`;

      debugFiles.push(dsymOutput);
    }
  }

  console.log('Uploading debug files to sentry');
  await $`yarn sentry-cli debug-files upload --include-sources --log-level=info packages`;

  for (const debugFile of debugFiles) {
    await $`rm -rf ${debugFile}`;
    console.log('Deleted', debugFile);
  }
}

uploadDebugSymbolsToSentry();
