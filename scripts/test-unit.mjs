import * as reporter from 'node:test/reporters';
import {run} from 'node:test';
import * as path from 'node:path';
import * as process from 'node:process';
import * as url from 'node:url';
import {finished} from 'node:stream';
import glob from 'glob';
// eslint-disable-next-line monorepo/no-internal-import
import { createRequire } from '@atlaspack/babel-register/createRequire.mjs';

const __filename = url.fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const __root = path.dirname(__dirname);

const require = createRequire(__filename);
globalThis.require = require;
console.log(globalThis.require( '/mnt/data/Development/atlassian-labs/atlaspack/packages/core/test-utils/test/fsFixture.test.js'))

// void (async function () {
//   const files = glob
//     .sync(
//       'packages/*/!(integration-tests|e2e-tests)/test/{*.{js,ts,cts,mts,cjs,mjs},**/*.{test,spec}.{js,ts,mts,cts,cjs,mjs}}',
//       {
//         cwd: __root,
//       },
//     )
//     .map((v) => path.join(__root, v));

//   let exitCode = 0;

//   const testStream = run({
//     files: [
//       '/mnt/data/Development/atlassian-labs/atlaspack/packages/core/test-utils/test/fsFixture.test.js',
//       // ...files
//     ],
//     // Runs the tests across the available CPUs
//     // Causes the tests to hang
//     forceExit: true,
//     concurrency: false,
//     only: true, //!!process.env.ONLY,
//     isolation: 'none',
//   })
//     .on('test:fail', () => {
//       // @ts-ignore
//       exitCode = 1;
//     })
//     .compose(new reporter.spec());

//   testStream.pipe(process.stdout);
//   await new Promise((res) => finished(testStream, res));
//   process.exit(exitCode);
// })();
