import {NextFunction, Request, Response} from 'express';
import {HTTPError} from '../../errors/HTTPError';

export function errorHandlingMiddleware(
  err: Error,
  _req: Request,
  res: Response,
  next: NextFunction,
) {
  if (err instanceof HTTPError) {
    res.status(err.status).json({error: err.message});
  } else {
    res.status(500).json({error: 'Internal server error'});
  }
}
