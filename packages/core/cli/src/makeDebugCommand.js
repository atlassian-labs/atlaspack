// @flow strict-local

import {NodeFS} from '@atlaspack/fs';
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

  const invalidate = debug
    .command('invalidate [input...]')
    .description('Run cache invalidation, then exit')
    .action(async (args: string[], opts: Options, command: CommandExt) => {
      try {
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
          ...options,
          shouldBuildLazily: true,
          watchBackend: 'watchman',
        });
        console.log('Created atlaspack instance');

        await atlaspack.unstable_invalidate();
        console.log('Done invalidating cache');
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
          ...options,
          shouldBuildLazily: true,
          watchBackend: 'watchman',
          featureFlags: {
            ...options.featureFlags,
            fixQuadraticCacheInvalidation: 'NEW',
            useLmdbJsLite: true,
          },
        });
        console.log('Created atlaspack instance');

        await atlaspack.unstable_buildAssetGraph();
        console.log('Done building asset graph');
        process.exit(0);
      } catch (err) {
        handleUncaughtException(err);
      }
    });
  applyOptions(buildAssetGraph, commonOptions);

  return debug;
}
