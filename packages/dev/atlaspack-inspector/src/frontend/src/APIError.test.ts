import assert from 'assert';
import {AxiosError} from 'axios';

import {APIError} from './APIError';

/**
 * Helper to create a minimal AxiosError-like object for testing purposes.
 */
function createAxiosError(
  method: string,
  url: string,
  status?: number,
  statusText?: string,
  data?: unknown,
  cause?: Error,
): AxiosError {
  return {
    name: 'AxiosError',
    message: 'mock',
    config: {method, url} as any,
    response: status ? ({status, statusText, data} as any) : undefined,
    cause,
    isAxiosError: true,
    toJSON() {
      return {};
    },
  } as unknown as AxiosError;
}

describe('APIError', function () {
  it('should create message including method, url, status, statusText and data', function () {
    const axiosErr = createAxiosError(
      'get',
      '/api/test',
      404,
      'Not Found',
      'Page not found',
    );

    const error = new APIError(axiosErr);

    assert(
      error.message.includes('GET /api/test 404 Not Found'),
      'Expected message to include HTTP method, url, and status',
    );
    assert(
      error.message.includes('Page not found'),
      'Expected message to include response data',
    );
    assert(error instanceof Error);
    assert(error instanceof APIError);
  });

  it('should include cause message when provided', function () {
    const cause = new Error('connection closed');
    const axiosErr = createAxiosError(
      'post',
      '/api/submit',
      500,
      'Internal Server Error',
      undefined,
      cause,
    );

    const error = new APIError(axiosErr);

    assert(
      error.message.includes('connection closed'),
      'Expected error message to include cause',
    );
  });
});
