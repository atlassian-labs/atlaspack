import assert from 'assert';
import {HTTPError} from './HTTPError';

describe('HTTPError', function () {
  it('should create HTTPError with message and status', function () {
    const error = new HTTPError('Not found', 404);

    assert.equal(error.message, 'Not found');
    assert.equal(error.status, 404);
    assert(error instanceof Error);
    assert(error instanceof HTTPError);
  });

  it('should be throwable', function () {
    assert.throws(() => {
      throw new HTTPError('Server error', 500);
    }, HTTPError);
  });
});
