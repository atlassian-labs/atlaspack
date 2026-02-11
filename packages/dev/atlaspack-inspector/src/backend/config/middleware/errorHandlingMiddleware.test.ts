import assert from 'assert';
import express from 'express';
import request from 'supertest';
import {errorHandlingMiddleware} from './errorHandlingMiddleware';
import {HTTPError} from '../../errors/HTTPError';

jest.mock('../logger');

describe('errorHandlingMiddleware integration', function () {
  let app: express.Express;

  beforeEach(() => {
    app = express();

    // Add test routes that throw errors
    app.get('/http-error', (req, res, next) => {
      next(new HTTPError('Custom not found', 404));
    });

    app.get('/server-error', (req, res, next) => {
      next(new Error('Something went wrong'));
    });

    app.get('/bad-request', (req, res, next) => {
      next(new HTTPError('Bad request error', 400));
    });

    // Add error handling middleware
    app.use(errorHandlingMiddleware);
  });

  it('should handle HTTPError with custom status and message', async () => {
    const response = await request(app).get('/http-error').expect(404);

    assert.deepEqual(response.body, {error: 'Custom not found', status: 404});
  });

  it('should handle regular Error as 500 internal server error', async () => {
    const response = await request(app).get('/server-error').expect(500);

    assert.deepEqual(response.body, {
      error: 'Internal server error',
      status: 500,
    });
  });

  it('should handle different HTTPError status codes', async () => {
    const response = await request(app).get('/bad-request').expect(400);

    assert.deepEqual(response.body, {error: 'Bad request error', status: 400});
  });
});
