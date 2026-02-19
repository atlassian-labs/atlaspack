/* eslint-disable import/first */
// Stateful import to ensure serializers are loaded
require('@atlaspack/core');

import {program} from 'commander';
import express from 'express';
import cors from 'cors';
import path from 'path';
import {setFeatureFlags, DEFAULT_FEATURE_FLAGS} from '@atlaspack/feature-flags';

import {CacheData, loadCacheData} from './services/loadCacheData';
import {logger} from './config/logger';
import {errorHandlingMiddleware} from './config/middleware/errorHandlingMiddleware';
import {loggingMiddleware} from './config/middleware/loggingMiddleware';
import {makeFrontendAssetsController} from './controllers/FrontendAssetsController';
import {cacheDataMiddleware} from './config/middleware/cacheDataMiddleware';
import {makeBundleGraphController} from './controllers/BundleGraphController';
import {makeTreemapController} from './controllers/TreeMapController';
import {makeCacheDataController} from './controllers/CacheDataController';
import {
  findProjectRoot,
  findRepositoryRoot,
  findSourceCodeURL,
  SourceCodeURL,
} from './services/findSourceCodeUrl';
import {AddressInfo} from 'net';
import {makeInspectorMCPController} from './controllers/mcp/InspectorMCPController';
import {spawn} from 'child_process';
import {analyticsService} from './services/AnalyticsService';

/**
 * We split preparing cache data and building the app.
 *
 * The cache is opened once and some models are created from it.
 *
 * These models are shared through the application on the `res.locals` express field.
 */
interface ConfigureInspectorAppParams {
  /**
   * A path to the cache directory or a path to a project.
   *
   * If a cache isn't found in this path, the tool will traverse up until it finds
   * a suitable root.
   *
   * Once a cache is found, a `.git` directory will be looked-up to find a "project root".
   *
   * This will be used to find files and to find source code URLs on GitHub or BitBucket
   * cloud.
   */
  target: string;
  /**
   * A path to the project root.
   *
   * If not provided, the tool will traverse up until it finds a suitable root.
   */
  projectRoot?: string;
}

/**
 * Configures the inspector app.
 *
 * - Find paths for the source repository, project and cache.
 * - Open the cache and deserialize bundler data out of it.
 * - Build the tree-map model.
 */
export function configureInspectorApp({
  projectRoot: projectRootFromFlags,
  target,
}: ConfigureInspectorAppParams): BuildInspectorAppParams {
  const flags = {
    ...DEFAULT_FEATURE_FLAGS,
  };
  setFeatureFlags(flags);

  const projectRoot =
    projectRootFromFlags ?? findProjectRoot(target) ?? path.dirname(target);
  const repositoryRoot = findRepositoryRoot(target) ?? path.dirname(target);

  const sourceCodeURL = findSourceCodeURL(target);

  logger.debug(
    {target, projectRoot, repositoryRoot, sourceCodeURL},
    'Found paths',
  );

  const cacheData = loadCacheData({target, projectRoot, repositoryRoot});

  return {
    cacheData,
    projectRoot,
    repositoryRoot,
    sourceCodeURL,
  };
}

export interface BuildInspectorAppParams {
  /**
   * A promise to the cache data.
   */
  cacheData: Promise<CacheData>;
  /**
   * The atlaspack project root.
   */
  projectRoot: string;
  /**
   * The repository root path for the target project. This will be used to link
   * to the source code on GitHub or BitBucket.
   */
  repositoryRoot: string;
  /**
   * The source code URL for the target project. The parsed remote URL.
   */
  sourceCodeURL: SourceCodeURL | null;
}

/**
 * Wire-up the express server app.
 */
export function buildInspectorApp({
  cacheData,
  projectRoot,
  repositoryRoot,
  sourceCodeURL,
}: BuildInspectorAppParams): express.Express {
  const app = express();

  app.use(loggingMiddleware());
  app.use(
    cors({
      origin: /http:\/\/localhost:(\d+)/,
      credentials: true,
    }),
  );
  app.use(express.json());

  app.use(cacheDataMiddleware(cacheData));
  app.use(makeFrontendAssetsController());
  app.use(makeBundleGraphController({projectRoot, repositoryRoot}));
  app.use(
    makeTreemapController({
      projectRoot,
      repositoryRoot,
      sourceCodeURL,
    }),
  );
  app.use(makeInspectorMCPController());
  app.use(makeCacheDataController());
  app.use(errorHandlingMiddleware);

  return app;
}

/**
 * Executes `atlaspack build` to build the client application for the inspector.
 *
 * @param targets - The targets/entry-points to build.
 */
async function buildClientApplicationForInspector(targets: string[]) {
  logger.info({targets}, 'Building app...');

  const child = spawn('yarn', ['atlaspack', 'build', ...targets], {
    shell: true,
    stdio: 'inherit',
  });

  await new Promise((resolve, reject) => {
    child.on('error', (error) => {
      logger.error(error, 'Build error');
      process.exitCode = 1;

      reject(error);
    });

    child.on('exit', (code) => {
      logger.info(`Build process exited with code ${code}`);
      if (code !== 0) {
        process.exitCode = code ?? 1;
        reject(new Error(`Build failed with code ${code}`));
      } else {
        resolve(null);
      }
    });
  });
}

/**
 * CLI entry point for `@atlaspack/inspector`.
 *
 * Usage:
 *
 * ```bash
 * yarn @atlaspack/inspector --target ./path/to/cache --project-root ./path/to/project
 * ```
 */
export function main() {
  const version = require('../../package.json').version;
  let isStartCommand = false;
  program.name('atlaspack-inspector').version(version);

  analyticsService.sendEvent({
    data: {
      name: 'atlaspack-inspector-start',
      action: 'atlaspack-inspector-start',
    },
  });

  const command = program
    .command('start [options]', {isDefault: true})
    .description('Start the inspector server')
    .option('-t, --target <path>', 'Path to the target cache', process.cwd())
    .option('-p, --port <port>', 'Port to run the server on', '3000')
    .option('--project-root <path>', 'Path to the project root', undefined)
    .action(() => {
      isStartCommand = true;
    });

  program
    .command('build <target...>')
    .description('Build an app with atlaspack-inspector required feature-flags')
    .action(async (targets) => {
      await buildClientApplicationForInspector(targets);
    });

  program.parse(process.argv);

  if (!isStartCommand) {
    return;
  }

  const options = command.opts();

  const inspectorAppParams = configureInspectorApp({
    target: options.target,
    projectRoot: options.projectRoot,
  });
  const app = buildInspectorApp(inspectorAppParams);

  const port = Number(options.port ?? process.env.PORT ?? 3000);

  const server = app.listen(port, () => {
    const address: AddressInfo | string | null = server.address();
    if (address == null) {
      logger.error('Server did not start correctly');
    } else {
      const addressString =
        typeof address === 'string'
          ? address
          : `http://localhost:${address.port}`;

      logger.info(`Server is running on ${addressString}`);
    }
  });
  server.on('error', (error) => {
    logger.error(error, 'HTTP Server error');
  });

  return server;
}
