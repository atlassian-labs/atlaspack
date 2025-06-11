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
  findSourceCodeURL,
  SourceCodeURL,
} from './services/findSourceCodeUrl';
import {AddressInfo} from 'net';

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
}

/**
 * - Find paths for the source repository, project and cache.
 * - Open the cache and deserialize bundler data out of it.
 * - Build the tree-map model.
 */
export async function configureInspectorApp({
  target,
}: ConfigureInspectorAppParams): Promise<BuildInspectorAppParams> {
  const flags = {
    ...DEFAULT_FEATURE_FLAGS,
    cachePerformanceImprovements: true,
  };
  setFeatureFlags(flags);

  const projectRoot = findProjectRoot(target) ?? path.dirname(target);
  const sourceCodeURL = findSourceCodeURL(projectRoot);
  const cacheData = await loadCacheData(target, projectRoot);

  return {
    cacheData,
    projectRoot,
    sourceCodeURL,
  };
}

export interface BuildInspectorAppParams {
  cacheData: CacheData;
  projectRoot: string;
  sourceCodeURL: SourceCodeURL | null;
}

/**
 * Wire-up the express server app.
 */
export function buildInspectorApp({
  cacheData,
  projectRoot,
  sourceCodeURL,
}: BuildInspectorAppParams): express.Express {
  const app = express();

  app.use(loggingMiddleware());
  app.use(
    cors({
      // origin: 'http://localhost:3333',
      credentials: true,
    }),
  );

  app.use(cacheDataMiddleware(cacheData));
  app.use(makeFrontendAssetsController());
  app.use(makeBundleGraphController({projectRoot}));
  app.use(
    makeTreemapController({
      sourceCodeURL,
    }),
  );
  app.use(makeCacheDataController());
  app.use(errorHandlingMiddleware);

  return app;
}

export async function main() {
  const command = program
    .requiredOption('-t, --target <path>', 'Path to the target cache')
    .option('-p, --port <port>', 'Port to run the server on', '3000')
    .parse(process.argv);

  const options = command.opts();

  const inspectorAppParams = await configureInspectorApp({
    target: options.target,
  });
  const app = buildInspectorApp(inspectorAppParams);

  const port = Number(options.port ?? process.env.PORT ?? 3000);
  const server = app.listen(port, () => {
    const address: AddressInfo = server.address() as AddressInfo;
    logger.info(`Server is running on http://localhost:${address.port}`);
  });

  return server;
}
