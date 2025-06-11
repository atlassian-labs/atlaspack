import path from 'path';
import express, {Request, Response, Router} from 'express';

/**
 * On production builds, we intend to serve a pre-built version of the front-end `dist`
 * directory in `src/frontend/dist`.
 *
 * On development mode, you should be running the front-end dev-server separately
 * and pointing it at this host.
 */
export function makeFrontendAssetsController() {
  const router = Router();

  router.use(
    express.static(path.join(__dirname, '../../../src/frontend/dist')),
  );
  router.use('/app/', (req, res, next) => {
    if (req.method === 'GET') {
      res.sendFile(
        path.join(__dirname, '../../../src/frontend/dist/index.html'),
      );
    } else {
      next();
    }
  });

  return router;
}
