// @flow strict-local

import {NodeFS} from '@atlaspack/fs';
import logger from '@atlaspack/logger';
import commander, {type commander$Command} from 'commander';
import path from 'path';
import {normalizeOptions, type Options} from './normalizeOptions';
import type {CommandExt} from './normalizeOptions';
import {applyOptions} from './applyOptions';
import {commonOptions} from './options';
import {handleUncaughtException} from './handleUncaughtException';
import type AtlaspackType from '@atlaspack/core';
import hapi from 'hapi';
import {serialize, deserialize} from '@atlaspack/build-cache';
import assert from 'assert';

async function runCacheInspector(atlaspack: AtlaspackType) {
  await atlaspack._init();
  const cache = atlaspack.unstable_getNativeCache();

  const app = hapi.Server({
    port: 9999,
    host: 'localhost',
    debug: {
      log: '*',
      request: '*',
    },
  });

  app.route({
    method: 'GET',
    path: '/api/keys',
    handler: (request, h) => {
      const q =
        request.query.q != null ? decodeURIComponent(request.query.q) : '';
      const page = request.query.page != null ? Number(request.query.page) : 0;
      let keys = cache.getNativeRef().listKeys();

      if (q && q !== '') {
        keys = keys.filter((key) => key.includes(q));
      }

      // const requestTypes = {
      //   '1': 'atlaspack_build_request',
      //   '2': 'bundle_graph_request',
      //   '3': 'asset_graph_request',
      //   '4': 'entry_request',
      //   '5': 'target_request',
      //   '6': 'atlaspack_config_request',
      //   '7': 'path_request',
      //   '8': 'dev_dep_request',
      //   '9': 'asset_request',
      //   '10': 'config_request',
      //   '11': 'write_bundles_request',
      //   '12': 'package_request',
      //   '13': 'write_bundle_request',
      //   '14': 'validation_request',
      // };
      // const requestsByType = {};
      // const requestGraphs = keys
      //   .filter((key) => key.startsWith('requestGraph:graph'))
      //   .map((key) => [key, cache.getSync(key)])
      //   .map(([key, graph]) => {
      //     const nodes = [];
      //     const cacheKey = key.split(':')[2];
      //     console.log(graph.nodeCount);
      //     for (let i = 0; i < graph.nodeCount; i++) {
      //       nodes.push(cache.getSync(`requestGraph:nodes:${cacheKey}:${i}`));
      //     }
      //     for (let node of nodes) {
      //       if (node == null) continue;
      //       if (node.type === 1) {
      //         requestsByType[requestTypes[node.requestType]] =
      //           (requestsByType[requestTypes[node.requestType]] || 0) + 1;
      //       }
      //     }
      //     return {
      //       ...graph,
      //       nodes,
      //     };
      //   });

      return {
        items: keys.slice(0 + page * 100, 100 + page * 100),
        total: keys.length,
        pageSize: 100,
        page: page,
        next: page + 1,
      };
    },
  });

  app.route({
    method: 'GET',
    path: '/api/keys/{key}',
    handler: async (request, h) => {
      const key = decodeURIComponent(request.params.key);
      const buffer = cache.getBlobSync(key);
      const isSmallValue = buffer.length < 1024 * 1024;
      let value = null;
      if (isSmallValue) {
        try {
          value = deserialize(buffer);
        } catch (err) {
          console.error(err);
          try {
            value = buffer.toString('utf8');
          } catch (err) {
            console.error(err);
          }
        }
      } else if (key.includes('AssetGraph')) {
        console.log('AssetGraph', key);
        try {
          value = deserialize(buffer);
        } catch (err) {
          console.error(err);
        }
        console.log('deserialized');

        console.time('write');
        const lmdb = cache.getNativeRef();
        await lmdb.startWriteTransaction();
        const envs = new Map();
        for (let node of value.result.assetGraph.nodes) {
          assert(node.id != null);
          if (node.type === 'asset') {
            envs.set(node.value.env.id, node.value.env);
            lmdb.putNoConfirm(
              `assetGraph:${key}:${node.type}:${node.value.filePath}:${node.id}`,
              serialize({
                ...node,
                value: {
                  ...node.value,
                  env: undefined,
                  envId: node.value.env.id,
                },
              }),
            );
          } else if (node.type === 'dependency') {
            envs.set(node.value.env.id, node.value.env);
            lmdb.putNoConfirm(
              `assetGraph:${key}:${node.type}:${node.value.sourcePath}:${node.value.specifier}:${node.id}`,
              serialize({
                ...node,
                value: {
                  ...node.value,
                  env: undefined,
                  envId: node.value.env.id,
                },
              }),
            );
          } else if (node.type === 'asset_group') {
            envs.set(node.value.env.id, node.value.env);
            lmdb.putNoConfirm(
              `assetGraph:${key}:${node.type}:${node.value.filePath}:${node.id}`,
              serialize({
                ...node,
                value: {
                  ...node.value,
                  env: undefined,
                  envId: node.value.env.id,
                },
              }),
            );
          } else {
            console.log(node.type);
            lmdb.putNoConfirm(
              `assetGraph:${key}:${node.type}:${node.id}`,
              serialize(node),
            );
          }
        }
        for (let env of envs.values()) {
          lmdb.putNoConfirm(`assetGraph:${key}:env:${env.id}`, serialize(env));
        }
        console.timeEnd('write');

        await lmdb.commitWriteTransaction();
      }

      return {
        key,
        valueSize: buffer.length,
        valueString: isSmallValue
          ? typeof value === 'string'
            ? value
            : JSON.stringify(value, null, 2)
          : 'Value too large to display',
      };
    },
  });

  await app.start();

  console.log('Cache inspector running at http://localhost:9999');
}

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

  const listCacheInvalidations = debug
    .command('list-cache-invalidations [input...]')
    .description('List cache invalidations, then exit')
    .action(async (args: string[], opts: Options, command: CommandExt) => {
      try {
        const atlaspack = await getInstance(args, opts, command);

        await atlaspack.unstable_listCacheInvalidations();
        process.exit(0);
      } catch (err) {
        handleUncaughtException(err);
      }
    });
  applyOptions(listCacheInvalidations, commonOptions);

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

  const bundlerStats = debug
    .command('bundler-stats [input...]')
    .description('Build the asset graph then exit')
    .action(async (entries: string[], opts: Options, command: CommandExt) => {
      try {
        const atlaspack = await getInstance(entries, opts, command);
        await atlaspack.unstable_getBundlerStats();
        logger.info({
          message: 'Done getting statistics',
          origin: '@atlaspack/cli',
        });
        process.exit(0);
      } catch (err) {
        handleUncaughtException(err);
      }
    });
  applyOptions(bundlerStats, commonOptions);

  const inspectCache = debug
    .command('inspect-cache [input...]')
    .description('Inspect the cache interactively')
    .action(async (entries: string[], opts: Options, command: CommandExt) => {
      const atlaspack = await getInstance(entries, opts, command);
      runCacheInspector(atlaspack);
    });
  applyOptions(inspectCache, commonOptions);

  return debug;
}
