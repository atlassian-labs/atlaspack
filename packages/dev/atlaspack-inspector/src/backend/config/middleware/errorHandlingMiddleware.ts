import {NextFunction, Request, Response} from 'express';
import {HTTPError} from '../../errors/HTTPError';
import {logger} from '../logger';

/**
 * A middleware that sets status codes for {@link HTTPError} instances.
 */
export function errorHandlingMiddleware(
  err: Error,
  _req: Request,
  res: Response,
  _next: NextFunction,
) {
  if (err instanceof HTTPError) {
    res.status(err.status).json({error: err.message, status: err.status});
  } else {
    logger.error(err, 'Internal server error');
    res.status(500).json({error: 'Internal server error', status: 500});
  }
}
