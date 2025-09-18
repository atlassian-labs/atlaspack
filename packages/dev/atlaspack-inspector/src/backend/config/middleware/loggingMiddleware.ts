import {logger} from '../logger';
import {Request, Response, RequestHandler, NextFunction} from 'express';

/**
 * Log HTTP status codes and durations with `pino`.
 */
export function loggingMiddleware(): RequestHandler {
  return (req: Request, res: Response, next: NextFunction) => {
    const start = Date.now();
    next();
    res.on('finish', () => {
      const duration = Date.now() - start;
      logger.debug(
        {
          method: req.method,
          statusCode: res.statusCode,
          url: req.url,
          duration,
        },
        `[${res.statusCode}] ${req.method} ${req.url} ${duration}ms`,
      );
    });
  };
}
