import pinoHttp, {HttpLogger} from 'pino-http';
import {logger} from '../logger';
import {Request, Response, RequestHandler} from 'express';

export function loggingMiddleware(): RequestHandler {
  return pinoHttp({
    logger,
    customLogLevel(_req: Request, res: Response, err?: Error | undefined) {
      if (err) {
        return 'error';
      } else if (res.statusCode >= 400) {
        return 'debug';
      }
      return 'debug';
    },
  });
}
