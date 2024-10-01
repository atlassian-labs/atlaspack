import {Command} from 'commander';

// @ts-expect-error - TS2732 - Cannot find module '../package.json'. Consider using '--resolveJsonModule' to import module with '.json' extension.
import packageJson from '../package.json';

import {migratePackageJson} from './migrations/migrate-package-json';
import {migrateParcelRc} from './migrations/migrate-parcelrc';

export async function run() {
  const program = new Command();

  program
    .name(packageJson.name)
    .description('Migrate from Parcel to Atlaspack')
    .option(
      '--cwd <cwd>',
      'The current working directory that the script will run the migration on',
      process.cwd(),
    )
    .option(
      '--dry-run',
      'Report the changes that will be made instead of making them',
    )
    .option(
      '--skip-dependencies',
      'Skip migrating the parcel dependencies to atlaspack',
    )
    .option(
      '--skip-engines',
      'Skip migrating the parcel key in package.json#engines',
    )
    .option(
      '--skip-parcelrc',
      'Skip migrating the .parcelrc file to .atlaspackrc',
    )
    .option(
      '--tag <tag>',
      'The tag used to search the npm registry for Atlaspack packages',
      'canary',
    )
    .version(packageJson.version);

  program.parse();

  const {
    cwd,
    dryRun,
    skipDependencies,
    skipEngines,
    skipParcelrc: skipParcelRc,
    tag,
  } = program.opts();

  // TODO Node API / types, parcel exe in scripts, etc
  await Promise.all([
    migratePackageJson({cwd, dryRun, skipDependencies, skipEngines, tag}),
    !skipParcelRc && migrateParcelRc({cwd, dryRun}),
  ]);
}
