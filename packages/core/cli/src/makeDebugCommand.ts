import {NodeFS} from '@atlaspack/fs';
import logger from '@atlaspack/logger';
import commander, {commander$Command} from 'commander';
import path from 'path';
import {normalizeOptions, Options} from './normalizeOptions';
import type {CommandExt} from './normalizeOptions';
import {applyOptions} from './applyOptions';
import {commonOptions} from './options';
import {handleUncaughtException} from './handleUncaughtException';

export function makeDebugCommand(): commander.Command {
  const debug = new commander.Command('debug').description(
    'Debug commands for atlaspack',
  );
  const getInstance = async (
    args: Array<string>,
    opts: Options,
    command: CommandExt,
  ) => {
    let entries = args;

    if (entries.length === 0) {
      entries = ['.'];
    }
    entries = entries.map((entry) => path.resolve(entry));

    Object.assign(command, opts);
    const fs = new NodeFS();
    const options = await normalizeOptions(command, fs);

    const Atlaspack = require('@atlaspack/core').default;

    const atlaspack = new Atlaspack({
      entries,
      defaultConfig: require.resolve('@atlaspack/config-default', {
        paths: [fs.cwd(), __dirname],
      }),
      shouldPatchConsole: false,
      shouldBuildLazily: false,
      ...options,
      watchBackend: 'watchman',
    });
    logger.info({
      message: 'Created atlaspack instance',
      origin: '@atlaspack/cli',
    });

    return atlaspack;
  };

  const invalidate = debug
    .command('invalidate [input...]')
    .description('Run cache invalidation, then exit')
    .action(async (args: string[], opts: Options, command: CommandExt) => {
      try {
        const atlaspack = await getInstance(args, opts, command);

        await atlaspack.unstable_invalidate();
        logger.info({
          message: 'Done invalidating cache',
          origin: '@atlaspack/cli',
        });
      } catch (err: any) {
        handleUncaughtException(err);
      }
    });
  applyOptions(invalidate, commonOptions);

  const buildAssetGraph = debug
    .command('build-asset-graph [input...]')
    .description('Build the asset graph then exit')
    .action(async (args: string[], opts: Options, command: CommandExt) => {
      try {
        const atlaspack = await getInstance(args, opts, command);

        await atlaspack.unstable_buildAssetGraph();
        logger.info({
          message: 'Done building asset graph',
          origin: '@atlaspack/cli',
        });
        process.exit(0);
      } catch (err: any) {
        handleUncaughtException(err);
      }
    });
  applyOptions(buildAssetGraph, commonOptions);

  const compactCache = debug
    .command('compact-cache [input...]')
    .description('Compact the cache')
    .action(async (args: string[], opts: Options, command: CommandExt) => {
      const atlaspack = await getInstance(args, opts, command);
      try {
        await atlaspack.unstable_compactCache();
        logger.info({
          message: 'Done compacting cache',
          origin: '@atlaspack/cli',
        });
        process.exit(0);
      } catch (err: any) {
        handleUncaughtException(err);
      }
    });
  applyOptions(compactCache, commonOptions);

  return debug;
}
