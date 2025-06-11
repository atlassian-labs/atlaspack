// Stateful import to ensure serializers are loaded
require('@atlaspack/core');

import {program} from 'commander';
import express from 'express';
import cors from 'cors';
import path from 'path';
import {setFeatureFlags, DEFAULT_FEATURE_FLAGS} from '@atlaspack/feature-flags';

import {loadCacheData} from './services/loadCacheData';
import {logger} from './config/logger';
import {errorHandlingMiddleware} from './config/middleware/errorHandlingMiddleware';
import {loggingMiddleware} from './config/middleware/loggingMiddleware';
import {makeFrontendAssetsController} from './controllers/FrontendAssetsController';
import {cacheDataMiddleware} from './config/middleware/cacheDataMiddleware';
import {makeBundleGraphController} from './controllers/BundleGraphController';
import {makeTreemapController} from './controllers/TreeMapController';
import {makeCacheDataController} from './controllers/CacheDataController';
import {findProjectRoot, findSourceCodeURL} from './services/findSourceCodeUrl';

export async function main() {
  const command = program
    .requiredOption('-t, --target <path>', 'Path to the target cache')
    .parse(process.argv);

  const flags = {
    ...DEFAULT_FEATURE_FLAGS,
    cachePerformanceImprovements: true,
  };
  setFeatureFlags(flags);

  const options = command.opts();

  const projectRoot =
    findProjectRoot(options.target) ?? path.dirname(options.target);
  const sourceCodeURL = findSourceCodeURL(projectRoot);
  const cacheData = await loadCacheData(options.target, projectRoot);

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

  app.listen(3000, () => {
    logger.info('Server is running on http://localhost:3000');
  });
}
