import {readFile, writeFile} from 'node:fs/promises';
import {basename, relative} from 'node:path';

import {fdir} from 'fdir';

import {diff} from './diff';
import {migrateConfigFields} from './package-json/migrate-config-fields';
import {migrateDependencies} from './package-json/migrate-dependencies';
import {migrateEnginesField} from './package-json/migrate-engines-field';

export type MigratePackageJsonOptions = {
  cwd: string;
  dryRun: string;
  skipDependencies: boolean;
  skipEngines: boolean;
  tag: string;
};

// TODO Update dependencies
export async function migratePackageJson({
  cwd,
  dryRun,
  skipDependencies,
  skipEngines,
  tag,
}: MigratePackageJsonOptions) {
  const {default: chalk} = await import('chalk');

  console.log(chalk.blue('[INFO]'), 'Searching for package.json files');

  const packageJsonPaths = await new fdir()
    .withFullPaths()
    .exclude(dir => dir.startsWith('node_modules'))
    .filter((path: string) => basename(path) === 'package.json')
    .crawl(cwd)
    .withPromise();

  const modifiedFiles = [];

  await Promise.all(
    packageJsonPaths.map(async packageJsonPath => {
      const rawPackageJson = await readFile(packageJsonPath, 'utf8');
      if (!rawPackageJson.includes('parcel')) {
        return;
      }

      const packageJson = JSON.parse(rawPackageJson);

      const didChange = [
        !skipEngines && migrateEnginesField(packageJson),
        migrateConfigFields(packageJson),
        !skipDependencies && migrateDependencies(packageJson, tag),
      ].includes(true);

      if (!didChange) {
        return;
      }

      const {space = 2} =
        rawPackageJson.match(/^\s*{.*\n(?<space>\s*)"/)?.groups ?? {};

      const migratedPackageJson =
        JSON.stringify(packageJson, null, space) + '\n';

      if (dryRun) {
        console.log(
          chalk.blue('[INFO]'),
          `Updated ${relative(cwd, packageJsonPath)}\n${diff(
            chalk,
            rawPackageJson,
            migratedPackageJson,
          )}`,
        );
      } else {
        await writeFile(packageJsonPath, migratedPackageJson);
      }

      modifiedFiles.push(packageJsonPath);
    }),
  );

  console.log(
    chalk.blue('[INFO]'),
    `Migrated ${modifiedFiles.length} package.json files`,
  );
}
