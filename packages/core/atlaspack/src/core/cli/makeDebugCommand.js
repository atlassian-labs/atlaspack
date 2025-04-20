// @flow strict-local

import {NodeFS} from '../fs/index.js';
import logger from '../logger/index.js';
import commander, {type commander$Command} from 'commander';
import path from 'path';
import {normalizeOptions, type Options} from './normalizeOptions';
import type {CommandExt} from './normalizeOptions';
import {applyOptions} from './applyOptions';
import {commonOptions} from './options';
import {handleUncaughtException} from './handleUncaughtException';

export function makeDebugCommand(): commander$Command {
  const debug = new commander.Command('debug').description(
    'Debug commands for atlaspack',
  );
  const getInstance = async (args, opts, command) => {
    let entries = args;

    if (entries.length === 0) {
      entries = ['.'];
    }
    entries = entries.map((entry) => path.resolve(entry));

    Object.assign(command, opts);
    const fs = new NodeFS();
    const options = await normalizeOptions(command, fs);

    const Atlaspack = require('../core/index').default;

    const atlaspack = new Atlaspack({
      entries,
      defaultConfig: require.resolve('@atlaspack/config-default', {
        paths: [fs.cwd(), __dirname],
      }),
      shouldPatchConsole: false,
      ...options,
      shouldBuildLazily: true,
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
      } catch (err) {
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
      } catch (err) {
        handleUncaughtException(err);
      }
    });
  applyOptions(buildAssetGraph, commonOptions);

  return debug;
}
