import {readFile, rename, writeFile} from 'node:fs/promises';
import {basename, dirname, join, relative} from 'node:path';

import {fdir} from 'fdir';

import {diff} from './diff';

export type MigrateParcelRcOptions = {
  cwd: string;
  dryRun: string;
};

// TODO: Support extended config defined in index.json?
export async function migrateParcelRc({cwd, dryRun}: MigrateParcelRcOptions) {
  const {default: chalk} = await import('chalk');

  console.log(chalk.blue('[INFO]'), 'Searching for .parcelrc files');

  const parcelrcPaths = await new fdir()
    .withFullPaths()
    .exclude((dir) => dir.startsWith('node_modules'))
    .filter((path: string) => basename(path).startsWith('.parcelrc'))
    .crawl(cwd)
    .withPromise();

  const modifiedFiles = [];

  await Promise.all(
    parcelrcPaths.map(async (parcelrcPath) => {
      const parcelrc = await readFile(parcelrcPath, 'utf8');
      const atlaspckrc = parcelrc.replace(/@parcel\//g, '@atlaspack/');
      if (atlaspckrc !== parcelrc) {
        if (dryRun) {
          console.log(
            chalk.blue('[INFO]'),
            `Updated ${relative(cwd, parcelrcPath)}\n${diff(
              chalk,
              parcelrc,
              atlaspckrc,
            )}`,
          );
        } else {
          await writeFile(parcelrcPath, atlaspckrc);
        }
      }

      const atlaspackrcPath = join(
        dirname(parcelrcPath),
        basename(parcelrcPath).replace('.parcelrc', '.atlaspackrc'),
      );

      if (dryRun) {
        console.log(
          chalk.blue('[INFO]'),
          `Renamed ${relative(cwd, parcelrcPath)}`,
        );
      } else {
        await rename(parcelrcPath, atlaspackrcPath);
      }

      modifiedFiles.push(parcelrcPath);
    }),
  );

  console.log(
    chalk.blue('[INFO]'),
    `Migrated ${modifiedFiles.length} .parcelrc files`,
  );
}
