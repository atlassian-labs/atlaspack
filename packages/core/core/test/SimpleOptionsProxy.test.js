// @noflow
const assert = require('assert');
const {optionsProxy} = require('../src/utils');

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

    const proxy = optionsProxy(options, invalidateOnOptionChange);

    // Access a property to trigger invalidation
    const value = proxy.hello;

    // Assert that the path is 'hello' and the value is 'world'
    assert.strictEqual(pathReceived, 'hello');
    assert.strictEqual(value, 'world');
  });
});
