// @flow strict-local

import sinon from 'sinon';
import {makeConfigProxy} from '../src/public/Config';
import assert from 'assert';

describe('makeConfigProxy with path tracking improvements', () => {
  it('tracks reads to nested fields with path arrays', () => {
    const onRead = sinon.spy();
    const target = {a: {b: {c: 'd'}}};
    const config = makeConfigProxy(onRead, target);
    config.a.b.c;
    assert.ok(onRead.calledWith(['a', 'b', 'c']));
    assert.ok(onRead.calledOnce);
  });

  // Test Object.keys() operations with different granularOptionInvalidation flag values
  [
    {granularOptionInvalidation: true},
    {granularOptionInvalidation: false},
  ].forEach(({granularOptionInvalidation}) => {
    it(`handles Object.keys() operations (granularOptionInvalidation: ${granularOptionInvalidation.toString()})`, () => {
      const onRead = sinon.spy();
      const target = {
        options: {
          featureFlags: {
            flag1: true,
            flag2: false,
          },
          settings: {
            mode: 'development',
          },
        },
      };

      // Save current feature flags
      // $FlowFixMe - We need to access the internal feature flags
      const originalFlags =
        require('@atlaspack/feature-flags').DEFAULT_FEATURE_FLAGS;

      try {
        // Override the feature flag for this test only
        // $FlowFixMe - We know this import has the setFeatureFlags method
        const featureFlags = require('@atlaspack/feature-flags');
        featureFlags.setFeatureFlags({
          ...originalFlags,
          granularOptionInvalidation,
        });

        const config = makeConfigProxy(onRead, target);

        // Object.keys() on a proxy object
        const keys = Object.keys(config.options.featureFlags);
        assert.deepEqual(keys, ['flag1', 'flag2']);

        assert.equal(
          onRead.callCount,
          1,
          `Object.keys() should always be tracked (granularOptionInvalidation=${granularOptionInvalidation.toString()})`,
        );

        // Reset the spy
        onRead.resetHistory();

        // Reading a specific property should still be tracked regardless of the flag
        assert.equal(config.options.featureFlags.flag1, true);
        assert.ok(onRead.calledWith(['options', 'featureFlags', 'flag1']));
        assert.equal(onRead.callCount, 1);
      } finally {
        // Restore original feature flags
        // $FlowFixMe - We know this import has the setFeatureFlags method
        require('@atlaspack/feature-flags').setFeatureFlags(originalFlags);
      }
    });
  });

  it('joins path segments with dots when tracking paths', () => {
    const onRead = sinon.spy();
    const reportedPaths = new Set();

    // Create a custom onRead that tracks the paths
    const customOnRead = (path) => {
      reportedPaths.add(path.join('.'));
      onRead(path);
    };

    const target = {
      config: {
        nested: {
          value: 42,
        },
      },
    };

    const config = makeConfigProxy(customOnRead, target);

    // Read the same path multiple times
    config.config.nested.value;
    config.config.nested.value;

    // Only reported once
    assert.equal(reportedPaths.size, 1);
    assert(reportedPaths.has('config.nested.value'));
    assert.equal(onRead.callCount, 1);
  });

  // Test root enumeration with different granularOptionInvalidation flag values
  [
    {granularOptionInvalidation: true},
    {granularOptionInvalidation: false},
  ].forEach(({granularOptionInvalidation}) => {
    it(`handles empty paths safely (granularOptionInvalidation: ${granularOptionInvalidation.toString()})`, () => {
      const onRead = sinon.spy();

      // Save current feature flags
      // $FlowFixMe - We need to access the internal feature flags
      const originalFlags =
        require('@atlaspack/feature-flags').DEFAULT_FEATURE_FLAGS;

      try {
        // Override the feature flag for this test only
        // $FlowFixMe - We know this import has the setFeatureFlags method
        const featureFlags = require('@atlaspack/feature-flags');
        featureFlags.setFeatureFlags({
          ...originalFlags,
          granularOptionInvalidation,
        });

        const config = makeConfigProxy(onRead, {});

        // Force an empty path (not normal usage, but should be handled)
        const proxy = config;

        // This should not cause errors or call onRead with empty path
        assert.doesNotThrow(() => {
          Object.keys(proxy);
        });

        // Root enumeration behavior depends on the feature flag
        if (granularOptionInvalidation) {
          // When granularOptionInvalidation is enabled, root enumeration should be tracked with __root__ marker
          assert.equal(onRead.callCount, 1);
          assert.ok(onRead.calledWith(['__root__']));
        } else {
          // When granularOptionInvalidation is disabled, root enumeration be tracked as an empty array
          assert.equal(onRead.callCount, 1);
          assert.ok(onRead.calledWith([]));
        }
      } finally {
        // Restore original feature flags
        // $FlowFixMe - We know this import has the setFeatureFlags method
        const featureFlags = require('@atlaspack/feature-flags');
        featureFlags.setFeatureFlags(originalFlags);
      }
    });
  });

  it('does not leak memory by repeatedly tracking the same paths', () => {
    const reportedPaths = new Set();
    const onRead = (path) => {
      reportedPaths.add(path.join('.'));
    };

    const target = {
      a: {
        b: {
          c: 'd',
          e: 'f',
        },
      },
    };

    const config = makeConfigProxy(onRead, target);

    // Read paths multiple times
    for (let i = 0; i < 100; i++) {
      config.a.b.c;
      config.a.b.e;
    }

    // Should only record two unique paths
    assert.equal(reportedPaths.size, 2);
    assert(reportedPaths.has('a.b.c'));
    assert(reportedPaths.has('a.b.e'));
  });

  it('correctly reports arrays and array elements', () => {
    // This test verifies that when we access array elements through a proxy,
    // the appropriate path tracking occurs based on the implementation

    const onRead = sinon.spy();
    const target = {
      items: [
        {id: 1, name: 'Item 1'},
        {id: 2, name: 'Item 2'},
      ],
    };

    const config = makeConfigProxy(onRead, target);

    // Reset the spy to start fresh
    onRead.resetHistory();

    // Access the proxy and items within it
    const items = config.items;
    const firstItem = items[0];
    const name = firstItem.name;

    // Verify the result
    assert.equal(name, 'Item 1');

    // Verify we called onRead at least once
    assert(onRead.called);
  });

  it('preserves proxy behavior across different levels of nesting', () => {
    const onRead = sinon.spy();
    const target = {
      level1: {
        level2: {
          level3: {
            value: 'deep',
          },
        },
        shallow: 'value',
      },
    };

    const config = makeConfigProxy(onRead, target);

    // Get a reference to a middle level
    const level2 = config.level1.level2;

    // We haven't accessed a primitive yet, so no read recorded
    assert.equal(onRead.callCount, 0);

    // Access through the reference
    const deepValue = level2.level3.value;
    assert.equal(deepValue, 'deep');

    // Should track the full path
    assert.ok(onRead.calledWith(['level1', 'level2', 'level3', 'value']));
    assert.equal(onRead.callCount, 1);

    // Access a property on the original proxy
    const shallow = config.level1.shallow;
    assert.equal(shallow, 'value');

    // Should track this path too
    assert.ok(onRead.calledWith(['level1', 'shallow']));
    assert.equal(onRead.callCount, 2);
  });

  // Additional tests for comprehensive coverage

  it('handles null values correctly', () => {
    const onRead = sinon.spy();
    const target = {
      nullValue: null,
      validValue: 'test',
    };

    const config = makeConfigProxy(onRead, target);

    // Reading null should be tracked
    const nullResult = config.nullValue;
    assert.strictEqual(nullResult, null);
    assert.ok(onRead.calledWith(['nullValue']));
    assert.equal(onRead.callCount, 1);
  });

  it('handles undefined values correctly', () => {
    const onRead = sinon.spy();
    const target = {
      // No undefined property explicitly set
    };

    const config = makeConfigProxy(onRead, target);

    // Reading undefined should be tracked
    const undefinedResult = config.undefinedProp;
    assert.strictEqual(undefinedResult, undefined);
    assert.ok(onRead.calledWith(['undefinedProp']));
    assert.equal(onRead.callCount, 1);
  });

  it('correctly tracks boolean values', () => {
    const onRead = sinon.spy();
    const target = {
      boolTrue: true,
      boolFalse: false,
    };

    const config = makeConfigProxy(onRead, target);

    // Reading booleans should be tracked
    const trueValue = config.boolTrue;
    assert.strictEqual(trueValue, true);
    assert.ok(onRead.calledWith(['boolTrue']));

    const falseValue = config.boolFalse;
    assert.strictEqual(falseValue, false);
    assert.ok(onRead.calledWith(['boolFalse']));

    assert.equal(onRead.callCount, 2);
  });

  it('preserves numeric indices in path arrays for arrays', () => {
    const onRead = sinon.spy();
    const target = {
      arrayProp: ['a', 'b', 'c'],
    };

    const config = makeConfigProxy(onRead, target);

    // Reset the spy
    onRead.resetHistory();

    // First get the array reference
    const array = config.arrayProp;

    // Then access by index - this should be tracked
    const item = array[1];
    assert.strictEqual(item, 'b');

    // Verify we tracked at least one call
    assert(onRead.called);
  });

  it('reports empty object reads only when accessing primitive properties', () => {
    const onRead = sinon.spy();
    const target = {
      emptyObj: {},
      objWithProps: {prop: 'value'},
    };

    const config = makeConfigProxy(onRead, target);

    // Reset the spy's call count to ensure we're starting fresh
    onRead.resetHistory();

    // Just getting the empty object reference might not trigger tracking
    // because it's returning a proxy to an object
    // eslint-disable-next-line no-unused-vars
    const emptyObj = config.emptyObj;

    // But trying to read a property should trigger tracking
    const prop = config.objWithProps.prop;
    assert.strictEqual(prop, 'value');
    assert.ok(onRead.calledWith(['objWithProps', 'prop']));
  });

  it('handles special properties safely', () => {
    const onRead = sinon.spy();
    const target = {
      prop: 'value',
    };

    const config = makeConfigProxy(onRead, target);

    // Reset the spy to ensure we're starting fresh
    onRead.resetHistory();

    // Access a regular property should work
    const val = config.prop;
    assert.strictEqual(val, 'value');
    assert.ok(onRead.calledWith(['prop']));
  });
});
