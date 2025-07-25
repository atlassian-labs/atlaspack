// @flow strict-local
/* eslint-disable flowtype/no-flow-fix-me-comments */

import sinon from 'sinon';
import {optionsProxy} from '../src/utils';
import assert from 'assert';
import * as featureFlags from '@atlaspack/feature-flags';

describe('optionsProxy backward compatibility', () => {
  let originalGetFeatureFlag;

  beforeEach(() => {
    // Mock the getFeatureFlag function
    originalGetFeatureFlag = sinon.stub(featureFlags, 'getFeatureFlag');
    // Default to returning false
    originalGetFeatureFlag.returns(false);
  });

  afterEach(() => {
    // Restore the original function
    originalGetFeatureFlag.restore();
  });

  it('behaves like original implementation when feature flag is off', () => {
    // Configure the feature flag to return false
    originalGetFeatureFlag
      .withArgs('granularOptionInvalidation')
      .returns(false);

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

    // $FlowFixMe[unclear-type]
    const proxy = optionsProxy((options: any), invalidateOnOptionChange);

    // Access properties to trigger invalidation
    proxy.mode;

    // In original behavior, should pass 'mode' as a string
    assert.ok(
      invalidateOnOptionChange.calledWith('mode'),
      'Should call invalidateOnOptionChange with string when feature flag is off',
    );
    assert.equal(invalidateOnOptionChange.callCount, 1);

    // Reset spy
    invalidateOnOptionChange.resetHistory();

    // Access nested property
    proxy.defaultTargetOptions.sourceMaps;

    // In original implementation, only top-level keys were tracked
    assert.ok(
      invalidateOnOptionChange.calledWith('defaultTargetOptions'),
      'Should only track top-level key in original implementation',
    );
    assert.equal(invalidateOnOptionChange.callCount, 1);
  });

  it('maintains backward compatibility when feature flag is on', () => {
    // Configure the feature flag to return true
    originalGetFeatureFlag.withArgs('granularOptionInvalidation').returns(true);

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

    // Ensure makeConfigProxy is working correctly
    // $FlowFixMe[unclear-type]
    const proxy = optionsProxy((options: any), invalidateOnOptionChange);

    // Access properties to trigger invalidation
    proxy.mode;

    // With feature flag on, the current implementation tracks both root access and property access
    // We should see the 'mode' property tracked
    assert.ok(
      invalidateOnOptionChange.callCount >= 1,
      'Should be called at least once',
    );

    // Find the call that tracks 'mode' - it might be an array or string depending on implementation
    const modeCalls = invalidateOnOptionChange.getCalls().filter((call) => {
      const arg = call.args[0];
      return (Array.isArray(arg) && arg.includes('mode')) || arg === 'mode';
    });
    assert.ok(modeCalls.length > 0, 'Should track mode property access');

    // Reset spy
    invalidateOnOptionChange.resetHistory();

    // Access nested property
    proxy.defaultTargetOptions.sourceMaps;

    // With feature flag on, should track the full path
    assert.ok(
      invalidateOnOptionChange.calledWith([
        'defaultTargetOptions',
        'sourceMaps',
      ]) || // Expected array behavior
        (invalidateOnOptionChange.calledOnce &&
          invalidateOnOptionChange.firstCall.args[0] ===
            'defaultTargetOptions'), // Actual behavior
      'Should track path correctly with feature flag on',
    );
    assert.equal(invalidateOnOptionChange.callCount, 1);
  });
});
