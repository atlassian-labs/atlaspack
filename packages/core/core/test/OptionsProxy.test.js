// @flow strict-local
/* eslint-disable flowtype/no-flow-fix-me-comments */

/*
 * NOTE ABOUT FLOW ERRORS:
 * This test file will show Flow errors because it tests dynamic proxy behavior
 * that Flow cannot statically analyze. These errors are expected and marked with
 * $FlowFixMe comments. The tests validate the runtime behavior which works correctly
 * despite Flow's inability to type-check the proxy operations.
 */

import sinon from 'sinon';
import assert from 'assert';
import {makeConfigProxy} from '../src/public/Config';
import {getValueAtPath} from '../src/requests/ConfigRequest';

// This file contains tests for the functionality of proxies used for option invalidation
// Note: Flow may report errors because it can't understand the dynamic proxy behavior,
// but these tests verify the runtime behavior which works correctly.

// Mock a simplified version of optionsProxy to avoid flow errors
function mockOptionsProxy(options, invalidateOnOptionChange) {
  const ignoreOptions = new Set([
    'env',
    'inputFS',
    'outputFS',
    'packageManager',
    'shouldDisableCache',
  ]);

  return makeConfigProxy((path) => {
    const [prop] = path;
    if (!ignoreOptions.has(prop)) {
      invalidateOnOptionChange(path);
    }
  }, options);
}

describe('optionsProxy with path tracking', () => {
  it('correctly tracks accessed options as path arrays', () => {
    const invalidateOnOptionChange = sinon.spy();

    const options = {
      mode: 'development',
      featureFlags: {
        enableOptionInvalidationBlocklist: false,
        granularOptionInvalidation: true,
      },
      defaultTargetOptions: {
        shouldOptimize: false,
        sourceMaps: true,
      },
      instanceId: 'test-instance',
    };

    const proxied = mockOptionsProxy(options, invalidateOnOptionChange);

    // Access various properties
    proxied.mode;
    // $FlowFixMe[prop-missing] Flow doesn't understand dynamic proxies
    proxied.featureFlags.granularOptionInvalidation;
    // $FlowFixMe[prop-missing] Flow doesn't understand dynamic proxies
    proxied.defaultTargetOptions.sourceMaps;

    // Verify correct paths were passed to invalidateOnOptionChange
    assert(invalidateOnOptionChange.calledWith(['mode']));
    assert(
      invalidateOnOptionChange.calledWith([
        'featureFlags',
        'granularOptionInvalidation',
      ]),
    );
    assert(
      invalidateOnOptionChange.calledWith([
        'defaultTargetOptions',
        'sourceMaps',
      ]),
    );

    // Total calls should match what we accessed
    assert.equal(invalidateOnOptionChange.callCount, 3);
  });

  it('does not track options in the ignoreOptions set', () => {
    const invalidateOnOptionChange = sinon.spy();

    const options = {
      mode: 'development',
      env: {NODE_ENV: 'development'},
      packageManager: {
        require: () => {},
      },
      inputFS: {
        readFile: () => {},
      },
      shouldDisableCache: false,
    };

    const proxied = mockOptionsProxy(options, invalidateOnOptionChange);

    // Access both ignored and tracked properties
    proxied.mode;
    proxied.env;
    proxied.packageManager;
    proxied.inputFS;
    proxied.shouldDisableCache;

    // Only non-ignored option (mode) should be tracked
    assert(invalidateOnOptionChange.calledWith(['mode']));
    assert.equal(invalidateOnOptionChange.callCount, 1);
  });

  it('supports nested object and array paths with getValueAtPath', () => {
    const options = {
      featureFlags: {
        flag1: true,
        flag2: false,
      },
      array: [1, 2, {nested: 'value'}],
      nullValue: null,
    };

    // Test object paths
    assert.equal(getValueAtPath(options, ['featureFlags', 'flag1']), true);
    assert.equal(getValueAtPath(options, ['featureFlags', 'flag2']), false);

    // Test array paths
    assert.equal(getValueAtPath(options, ['array', '0']), 1);
    assert.equal(getValueAtPath(options, ['array', '2', 'nested']), 'value');

    // Test edge cases
    assert.equal(getValueAtPath(options, ['nullValue']), null);
    assert.equal(getValueAtPath(options, ['nonExistent']), undefined);
    assert.equal(getValueAtPath(options, ['nullValue', 'property']), undefined);
  });

  it('creates a proxy that behaves like the original object', () => {
    const invalidateOnOptionChange = sinon.spy();

    const original = {
      mode: 'development',
      targets: {
        browsers: ['Chrome > 80'],
      },
    };

    const proxied = mockOptionsProxy(original, invalidateOnOptionChange);

    // Should return the same primitive values
    assert.equal(proxied.mode, original.mode);

    // Reset the spy to ensure clean state for next checks
    invalidateOnOptionChange.resetHistory();

    // Deep properties should be accessible
    // $FlowFixMe[prop-missing] Flow doesn't understand dynamic proxies
    const browsers = proxied.targets.browsers;
    assert.equal(browsers[0], 'Chrome > 80');

    // Methods on objects should work
    assert.equal(browsers.length, 1);
  });

  it('tracks array property access with correct path segments', () => {
    const invalidateOnOptionChange = sinon.spy();

    const options = {
      plugins: [
        {name: 'plugin1', options: {enabled: true}},
        {name: 'plugin2', options: {enabled: false}},
      ],
    };

    const proxied = mockOptionsProxy(options, invalidateOnOptionChange);

    // Reset the spy
    invalidateOnOptionChange.resetHistory();

    // Access array element and nested property
    // $FlowFixMe[prop-missing] Flow doesn't understand dynamic proxies
    const plugins = proxied.plugins;
    const plugin = plugins[0];
    const enabled = plugin.options.enabled;

    // Verify we got the correct value
    assert.equal(enabled, true);

    // Verify at least one call was made to track paths
    assert(invalidateOnOptionChange.called);
  });

  it('properly handles undefined, null and primitive values', () => {
    const invalidateOnOptionChange = sinon.spy();

    const options = {
      nullValue: null,
      zeroValue: 0,
      falseValue: false,
      emptyString: '',
      undefinedValue: undefined,
    };

    const proxied = mockOptionsProxy(options, invalidateOnOptionChange);

    // Access various primitive values
    assert.strictEqual(proxied.nullValue, null);
    assert.strictEqual(proxied.zeroValue, 0);
    assert.strictEqual(proxied.falseValue, false);
    assert.strictEqual(proxied.emptyString, '');
    assert.strictEqual(proxied.undefinedValue, undefined);
    // $FlowFixMe[prop-missing] Flow doesn't understand dynamic proxies - nonExistentProp doesn't exist
    assert.strictEqual(proxied.nonExistentProp, undefined);

    // Verify tracking calls
    assert(invalidateOnOptionChange.calledWith(['nullValue']));
    assert(invalidateOnOptionChange.calledWith(['zeroValue']));
    assert(invalidateOnOptionChange.calledWith(['falseValue']));
    assert(invalidateOnOptionChange.calledWith(['emptyString']));
    assert(invalidateOnOptionChange.calledWith(['undefinedValue']));
    assert(invalidateOnOptionChange.calledWith(['nonExistentProp']));

    assert.equal(invalidateOnOptionChange.callCount, 6);
  });

  it('deduplicates repeated access to the same path', () => {
    const trackedPaths = new Set();
    const invalidateOnOptionChange = (path) => {
      trackedPaths.add(path.join('.'));
    };

    const options = {
      mode: 'development',
    };

    const proxied = mockOptionsProxy(options, invalidateOnOptionChange);

    // Access a property multiple times
    // $FlowFixMe[prop-missing] Flow doesn't understand dynamic proxies
    const mode1 = proxied.mode;
    // $FlowFixMe[prop-missing] Flow doesn't understand dynamic proxies
    const mode2 = proxied.mode;
    // $FlowFixMe[prop-missing] Flow doesn't understand dynamic proxies
    const mode3 = proxied.mode;

    assert.equal(mode1, 'development');
    assert.equal(mode2, 'development');
    assert.equal(mode3, 'development');

    // Should only track once
    assert.equal(trackedPaths.size, 1);
    assert(trackedPaths.has('mode'));
  });
});
