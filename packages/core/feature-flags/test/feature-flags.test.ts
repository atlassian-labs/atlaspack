import assert from 'assert';
import {
  getFeatureFlag,
  DEFAULT_FEATURE_FLAGS,
  setFeatureFlags,
  runWithConsistencyCheck,
} from '../src';
import sinon from 'sinon';

describe('feature-flag test', () => {
  beforeEach(() => {
    setFeatureFlags(DEFAULT_FEATURE_FLAGS);
  });

  it('has defaults', () => {
    assert.equal(
      getFeatureFlag('exampleFeature'),
      DEFAULT_FEATURE_FLAGS.exampleFeature,
    );
  });

  it('can override', () => {
    setFeatureFlags({...DEFAULT_FEATURE_FLAGS, exampleFeature: true});
    assert.equal(getFeatureFlag('exampleFeature'), true);
  });

  describe('consistency checks', () => {
    it('runs the old function if the flag is off', () => {
      setFeatureFlags({
        ...DEFAULT_FEATURE_FLAGS,
        exampleConsistencyCheckFeature: 'OLD',
      });
      const result = runWithConsistencyCheck(
        'exampleConsistencyCheckFeature',
        () => 'old',
        () => 'new',
        sinon.spy(),
        sinon.spy(),
      );
      assert.equal(result, 'old');
    });

    it('runs the new function if the flag is on', () => {
      setFeatureFlags({
        ...DEFAULT_FEATURE_FLAGS,
        exampleConsistencyCheckFeature: 'NEW',
      });
      const result = runWithConsistencyCheck(
        'exampleConsistencyCheckFeature',
        () => 'old',
        () => 'new',
        sinon.spy(),
        sinon.spy(),
      );
      assert.equal(result, 'new');
    });

    it('diffs old and new values if there is a diff value', () => {
      setFeatureFlags({
        ...DEFAULT_FEATURE_FLAGS,
        exampleConsistencyCheckFeature: 'OLD_AND_CHECK',
      });
      const reportSpy = sinon.spy();
      const result = runWithConsistencyCheck(
        'exampleConsistencyCheckFeature',
        () => 'old',
        () => 'new',
        () => ({isDifferent: false, custom: 'diff'}),
        reportSpy,
      );

      assert.equal(result, 'old');
      sinon.assert.calledWith(reportSpy, {
        isDifferent: false,
        oldExecutionTimeMs: sinon.match.number,
        newExecutionTimeMs: sinon.match.number,
        custom: 'diff',
      });
    });

    it('diffs old and new values if there is a diff new value', () => {
      setFeatureFlags({
        ...DEFAULT_FEATURE_FLAGS,
        exampleConsistencyCheckFeature: 'NEW_AND_CHECK',
      });
      const reportSpy = sinon.spy();
      const result = runWithConsistencyCheck(
        'exampleConsistencyCheckFeature',
        () => 'old',
        () => 'new',
        () => ({isDifferent: true, custom: 'diff'}),
        reportSpy,
      );

      assert.equal(result, 'new');
      sinon.assert.calledWith(reportSpy, {
        isDifferent: true,
        oldExecutionTimeMs: sinon.match.number,
        newExecutionTimeMs: sinon.match.number,
        custom: 'diff',
      });
    });
  });
});
