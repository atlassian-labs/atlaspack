// @flow strict
import assert from 'assert';
import sinon from 'sinon';
import {
  setFeatureFlags,
  getFeatureFlag,
  getFeatureFlagValue,
  resetFlags,
  getAccessedFlags,
} from '../feature-flag-mock.js';

describe('feature-flag-mock', () => {
  let originalEnv;

  beforeEach(() => {
    originalEnv = {...process.env};
    resetFlags();
  });

  afterEach(() => {
    process.env = originalEnv;
  });

  // [] TODO: if using RANDOM_GATES or ALL_ENABLED in module override - packages/core/test-utils/src/feature-flag-override.js - rewrite this test
  describe('when no environment flags are set', () => {
    it('should return default feature flag values', () => {
      assert.equal(getFeatureFlag('exampleFeature'), false);
      assert.equal(
        getFeatureFlagValue('exampleConsistencyCheckFeature'),
        'OLD',
      );
    });
  });

  describe('when RANDOM_GATES is true', () => {
    beforeEach(() => {
      process.env.RANDOM_GATES = 'true';
    });

    it('should return mocked values if previously set', () => {
      const flags = {
        exampleFeature: true,
        exampleConsistencyCheckFeature: 'NEW',
      };

      setFeatureFlags(flags);

      assert.equal(getFeatureFlag('exampleFeature'), true);
      assert.equal(getFeatureFlag('exampleConsistencyCheckFeature'), true);
    });

    it('should randomise to string when unset flag is string type', () => {
      const result = getFeatureFlagValue('exampleConsistencyCheckFeature');

      assert.equal(typeof result, 'string');
      assert(['NEW', 'OLD', 'NEW_AND_CHECK', 'OLD_AND_CHECK'].includes(result));
    });

    it('should randomise to boolean when unset flag is boolean type', () => {
      const mathRandomSpy = sinon.spy(Math, 'random');

      try {
        const result = getFeatureFlagValue('exampleFeature');

        assert.equal(typeof result, 'boolean');
        assert(mathRandomSpy.called, 'Math.random should have been called');
      } finally {
        mathRandomSpy.restore();
      }
    });

    it('should return consistent randomised values on multiple calls', () => {
      const firstCall = getFeatureFlagValue('consistentFlag');
      const secondCall = getFeatureFlagValue('consistentFlag');

      assert.equal(firstCall, secondCall);
    });
  });

  describe('when ALL_ENABLED is true', () => {
    beforeEach(() => {
      process.env.ALL_ENABLED = 'true';
    });

    it('should return mocked values if previously set', () => {
      setFeatureFlags({exampleFeature: false});
      assert.equal(getFeatureFlag('exampleFeature'), false);
    });

    it('should return true for all flags, unless mocked', () => {
      setFeatureFlags({exampleFeature: false});
      assert.equal(getFeatureFlag('exampleFeature'), false);
      assert.equal(getFeatureFlag('exampleConsistencyCheckFeature'), true);
    });
  });

  describe('resetFlags', () => {
    it('should clear all mocked flags', () => {
      setFeatureFlags({exampleFeature: true});
      getFeatureFlag('exampleFeature');

      resetFlags();

      const accessedFlags = getAccessedFlags();
      assert.equal(accessedFlags.size, 0);
    });
  });
});
