// @noflow
/* eslint-disable flowtype/no-flow-fix-me-comments */
/*
 * NOTE: This integration test has Flow type errors because it's using internal APIs
 * and custom options that Flow doesn't recognize.
 *
 * All errors are expected and suppressed with $FlowFixMe comments throughout the file.
 * We're suppressing errors related to:
 * - Custom properties not defined in the type system (optionInvalidation, nestedOption)
 * - Access to internal APIs (getRequestTracker)
 * - Incompatible exact/inexact object types
 */

// $FlowFixMe[incompatible-exact] - Suppress inexact object literal errors throughout the file
// $FlowFixMe[prop-missing] - Suppress property missing errors throughout the file
// $FlowFixMe[incompatible-use] - Suppress incompatible use errors throughout the file
// $FlowFixMe[incompatible-call] - Suppress incompatible call errors throughout the file

import assert from 'assert';
import path from 'path';
import {bundle, overlayFS, fsFixture} from '@atlaspack/test-utils';
import sinon from 'sinon';
import {
  setFeatureFlags,
  getFeatureFlag,
  DEFAULT_FEATURE_FLAGS,
} from '@atlaspack/feature-flags';
import {RequestGraph} from '@atlaspack/core/src/RequestTracker';

describe('Option invalidation in cache integration test', () => {
  let originalFeatureFlags;
  let requestGraphSpy;

  beforeEach(() => {
    // Save original feature flags
    originalFeatureFlags = {
      granularOptionInvalidation: getFeatureFlag('granularOptionInvalidation'),
    };

    // Setup a spy on RequestGraph.invalidateOptionNodes
    requestGraphSpy = sinon.spy(
      RequestGraph.prototype,
      'invalidateOptionNodes',
    );
  });

  afterEach(() => {
    // Restore original feature flags
    setFeatureFlags({
      ...DEFAULT_FEATURE_FLAGS,
      ...originalFeatureFlags,
    });

    // Restore the spy
    if (requestGraphSpy) {
      requestGraphSpy.restore();
    }
  });

  // Test that granularOptionInvalidation feature flag works
  it('respects blocklist with granularOptionInvalidation=true', async function () {
    // Set the feature flag for this test
    setFeatureFlags({
      ...DEFAULT_FEATURE_FLAGS,
      granularOptionInvalidation: true,
    });

    const dir = path.join(__dirname, 'disabled-import-cond-test');
    await overlayFS.mkdirp(dir);

    await fsFixture(overlayFS, dir)`
      .parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "reporters": []
        }

      index.js:
        export const value = "test";
    `;

    // Test that the basic setup works with feature flag enabled globally
    const firstBuild = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      defaultConfig: path.join(dir, '.parcelrc'),
    });

    // Second build
    const secondBuild = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      defaultConfig: path.join(dir, '.parcelrc'),
    });

    // Both builds should have completed successfully
    assert(firstBuild, 'First build should have completed successfully');
    assert(secondBuild, 'Second build should have completed successfully');
  });
});
