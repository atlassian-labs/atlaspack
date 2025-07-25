// @flow strict-local
/* eslint-disable flowtype/no-flow-fix-me-comments */

import sinon from 'sinon';
import assert from 'assert';

// Test proxy implementation without feature flags
function createOptionsProxy(
  options,
  invalidateOnOptionChange,
  useGranularTracking,
) {
  const packageManager = options.packageManager;

  // Function to create nested proxies (moved to function body root)
  function createNestedProxy(obj, currentPath = []) {
    return new Proxy(obj, {
      get(target, prop) {
        if (prop === 'packageManager') {
          return packageManager;
        }

        const newPath = [...currentPath, prop];

        if (typeof target[prop] === 'object' && target[prop] !== null) {
          return createNestedProxy(target[prop], newPath);
        }

        invalidateOnOptionChange(newPath);
        return target[prop];
      },
    });
  }

  if (useGranularTracking) {
    // Create a proxy that tracks paths as arrays
    return createNestedProxy(options);
  } else {
    // Original behavior - only track top-level props as strings
    return new Proxy(options, {
      get(target, prop) {
        if (prop === 'packageManager') {
          return packageManager;
        }

        invalidateOnOptionChange(String(prop));
        return target[prop];
      },
    });
  }
}

describe('Options Proxy Behavior', () => {
  it('tracks only top-level property names as strings with legacy behavior', () => {
    const invalidateOnOptionChange = sinon.spy();

    const options = {
      mode: 'development',
      defaultTargetOptions: {
        sourceMaps: true,
      },
      packageManager: {
        require: () => ({}),
      },
    };

    const proxy = createOptionsProxy(options, invalidateOnOptionChange, false);

    // Access properties to trigger invalidation
    proxy.mode;

    // With legacy behavior, should pass 'mode' as a string
    assert.ok(
      invalidateOnOptionChange.calledWith('mode'),
      'Should call invalidateOnOptionChange with string in legacy mode',
    );
    assert.equal(invalidateOnOptionChange.callCount, 1);

    // Reset spy
    invalidateOnOptionChange.resetHistory();

    // Access nested property
    proxy.defaultTargetOptions.sourceMaps;

    // In original implementation, only top-level keys were tracked
    assert.ok(
      invalidateOnOptionChange.calledWith('defaultTargetOptions'),
      'Should only track top-level key in legacy mode',
    );
    assert.equal(invalidateOnOptionChange.callCount, 1);
  });

  it('tracks full property paths as arrays with granular tracking', () => {
    const invalidateOnOptionChange = sinon.spy();

    const options = {
      mode: 'development',
      defaultTargetOptions: {
        sourceMaps: true,
      },
      packageManager: {
        require: () => ({}),
      },
    };

    const proxy = createOptionsProxy(options, invalidateOnOptionChange, true);

    // Access properties to trigger invalidation
    proxy.mode;

    // With granular tracking, should pass ['mode'] as an array
    assert.ok(
      invalidateOnOptionChange.calledWith(['mode']),
      'Should call invalidateOnOptionChange with array path in granular mode',
    );
    assert.equal(invalidateOnOptionChange.callCount, 1);

    // Reset spy
    invalidateOnOptionChange.resetHistory();

    // Access nested property
    proxy.defaultTargetOptions.sourceMaps;

    // With granular tracking, should track the full path
    assert.ok(
      invalidateOnOptionChange.calledWith([
        'defaultTargetOptions',
        'sourceMaps',
      ]),
      'Should track full path in granular mode',
    );
    assert.equal(invalidateOnOptionChange.callCount, 1);
  });
});
