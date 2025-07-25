// @flow strict-local
/* eslint-disable flowtype/no-flow-fix-me-comments */

import assert from 'assert';
import {optionsProxy} from '../src/utils';

describe('Basic optionsProxy test', () => {
  it('should behave correctly with string paths', () => {
    const options = {
      mode: 'development',
      hello: 'world',
      packageManager: {test: true},
    };

    let pathReceived = null;

    const invalidateOnOptionChange = (path) => {
      pathReceived = path;
      // With feature flags off, this should be a string
      assert.strictEqual(
        typeof path,
        'string',
        'Path should be a string with feature flags off',
      );
    };

    // $FlowFixMe[unclear-type]
    const proxy = optionsProxy((options: any), invalidateOnOptionChange);

    // Access a property to trigger invalidation
    // $FlowFixMe[unclear-type]
    const value = (proxy: any).hello;

    // Assert that the path is 'hello' and the value is 'world'
    assert.strictEqual(pathReceived, 'hello');
    assert.strictEqual(value, 'world');
  });
});
