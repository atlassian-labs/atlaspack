/* eslint-disable no-console */

/*
  This script goes over the source flow files and generates
  TypeScript declarations
*/

import * as path from 'node:path';
import * as fs from 'node:fs';
import glob from 'glob';
import {spawn} from './utils/spawn.mts';
import {Paths} from './utils/paths.mts';
import {createDirAll, writeString} from './utils/fs-extra.mts';
import {flowToFlowFixFile} from './utils/flow-to-ts-fix.mts';

const bins = {
  flowToTs: path.join(Paths.node_modules, '.bin', 'flow-to-ts'),
  tsc: path.join(Paths.node_modules, '.bin', 'tsc'),
};

async function main() {
  const pendingFlowToTs: Array<Promise<void>> = [];

  for (const foundRel of glob.sync('**/*.js', {
    cwd: Paths.unifiedSrc,
    ignore: ['**/*.test.js', '**/node_modules/**', '**/vendor/**'],
  })) {
    async function flowToTs() {
      const foundAbs = path.join(Paths.unifiedSrc, foundRel);
      const outputTypeScript = path
        .join(Paths.unifiedDist, foundRel)
        .replace('.js', '.ts');

      await createDirAll(path.dirname(outputTypeScript));

      const result = await spawn('node', [bins.flowToTs, foundAbs], {
        cwd: Paths.unifiedSrc,
        shell: true,
      });

      await writeString(outputTypeScript, result);

      await spawn(
        'node',
        [
          bins.tsc,
          '--emitDeclarationOnly',
          '--declaration',
          '--esModuleInterop',
          outputTypeScript,
        ],
        {
          cwd: Paths.unifiedSrc,
          shell: true,
        },
      );

      await fs.promises.rm(outputTypeScript);
    }

    pendingFlowToTs.push(new Promise((res) => setTimeout(res)).then(flowToTs));
  }

  await Promise.all(pendingFlowToTs);

  for (const foundRel of glob.sync('**/*.d.ts', {
    cwd: Paths.unifiedDist,
    ignore: ['**/*.test.js', '**/node_modules/**', '**/vendor/**'],
  })) {
    const foundAbs = path.join(Paths.unifiedDist, foundRel);
    await flowToFlowFixFile(foundAbs);
  }
}

main().catch((error) => {
  console.log(error);
  process.exit(1);
});
