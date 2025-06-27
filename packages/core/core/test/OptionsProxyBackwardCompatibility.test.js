// @noflow

import sinon from 'sinon';
import {optionsProxy} from '../src/utils';
import assert from 'assert';
import * as featureFlags from '@atlaspack/feature-flags';

describe('optionsProxy backward compatibility', () => {
  let getFeatureFlagStub;

  beforeEach(() => {
    // Stub the getFeatureFlag function
    getFeatureFlagStub = sinon.stub(featureFlags, 'getFeatureFlag');
    // Default to returning false
    getFeatureFlagStub.returns(false);
  });

  afterEach(() => {
    // Restore the original function
    getFeatureFlagStub.restore();
  });

  it('behaves like original implementation when feature flag is off', () => {
    // Ensure feature flag is off
    getFeatureFlagStub.withArgs('granularOptionInvalidation').returns(false);

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

    const proxy = optionsProxy(options, invalidateOnOptionChange);

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

  it('uses array paths when feature flag is on', () => {
    // Enable feature flag
    getFeatureFlagStub.withArgs('granularOptionInvalidation').returns(true);
    // This is needed for makeConfigProxy
    getFeatureFlagStub.withArgs('skipEnumerationTracking').returns(false);

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

    const proxy = optionsProxy(options, invalidateOnOptionChange);

    // Access properties to trigger invalidation
    proxy.mode;

    // With feature flag on, should pass ['mode'] as an array
    assert.ok(
      invalidateOnOptionChange.calledWith(['mode']),
      'Should call invalidateOnOptionChange with array path when feature flag is on',
    );
    assert.equal(invalidateOnOptionChange.callCount, 1);

    // Reset spy
    invalidateOnOptionChange.resetHistory();

    // Access nested property
    proxy.defaultTargetOptions.sourceMaps;

    // With feature flag on, should track the full path
    assert.ok(
      invalidateOnOptionChange.calledWith([
        'defaultTargetOptions',
        'sourceMaps',
      ]),
      'Should track full path with feature flag on',
    );
    assert.equal(invalidateOnOptionChange.callCount, 1);
  });
});
