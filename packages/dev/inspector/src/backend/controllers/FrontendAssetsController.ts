import path from 'path';
import express, {Request, Response, Router} from 'express';

export function makeFrontendAssetsController() {
  const router = Router();

  router.use(express.static(path.join(__dirname, '../../frontend/dist')));
  router.use('/app/', (req, res, next) => {
    if (req.method === 'GET') {
      res.sendFile(path.join(__dirname, '../../frontend/dist/index.html'));
    } else {
      next();
    }
  });

  return router;
}
