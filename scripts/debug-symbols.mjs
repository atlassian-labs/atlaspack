/* eslint-disable no-console */
import * as path from 'node:path';
import * as fs from 'node:fs';
import * as process from 'node:process';
import * as url from 'node:url';
import {$} from 'zx';
import glob from 'glob';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const __root = path.dirname(__dirname);

const rm = process.argv.includes('--rm');
const upload = process.argv.includes('--upload-to-sentry');

console.table({
  'Strip Debug Symbols': true,
  'Remove After': rm,
  'Upload to Sentry': upload,
});

void (async function main() {
  const debugFiles = await stripDebugSymbols();
  if (upload) {
    await uploadDebugSymbolsToSentry();
  }
  if (rm) {
    await removeFiles(debugFiles);
  }
})();

/// Find cdylib files and extract their debug symbols into
/// a separate debug file
async function stripDebugSymbols() {
  $.stdio = 'inherit';

  const debugFiles = [];

  for (const foundRel of glob.sync('packages/**/*.node', {
    cwd: __root,
    ignore: '**/node_modules/**',
  })) {
    const found = path.join(__root, foundRel);
    console.log('Stripping', found);

    if (process.platform === 'linux') {
      const output = `${found}.debug`;
      console.log('Generating', output);

      await $`objcopy --only-keep-debug --compress-debug-sections=zlib ${found} ${output}`;
      await $`objcopy --strip-debug --strip-unneeded ${found}`;
      await $`objcopy --add-gnu-debuglink=${output} ${found}`;

      debugFiles.push(output);
    } else if (process.platform === 'darwin') {
      const dsymOutput = `${found}.dsym`;
      console.log('Generating', dsymOutput);

      await $`dsymutil ${found} -o ${dsymOutput}`;
      await $`strip -x ${found}`;

      debugFiles.push(dsymOutput);
    }
  }

  return debugFiles;
}

/// Upload a list of debug files to sentry
async function uploadDebugSymbolsToSentry() {
  $.stdio = 'inherit';
  console.log('Uploading debug files to sentry');
  await $`yarn sentry-cli debug-files upload --include-sources --log-level=info packages`;
}

/// Remove/clean up debug files
async function removeFiles(debugFiles) {
  for (const debugFile of debugFiles) {
    console.log('Deleting', debugFile);
    await fs.promises.rm(debugFile, {recursive: true, force: true});
    console.log('Deleted', debugFile);
  }
}
