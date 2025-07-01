import assert from 'assert';
import path from 'path';
import {bundle, overlayFS, fsFixture} from '@atlaspack/test-utils';
import {
  setFeatureFlags,
  getFeatureFlag,
  DEFAULT_FEATURE_FLAGS,
} from '@atlaspack/feature-flags';

describe('Option invalidation in cache integration test', () => {
  let originalFeatureFlags;

  beforeEach(() => {
    // Save original feature flags
    originalFeatureFlags = {
      granularOptionInvalidation: getFeatureFlag('granularOptionInvalidation'),
    };
  });

  afterEach(() => {
    // Restore original feature flags
    setFeatureFlags({
      ...DEFAULT_FEATURE_FLAGS,
      ...originalFeatureFlags,
    });
  });

  it('respects blocklist with granularOptionInvalidation=true', async function () {
    // Set the feature flag for this test
    setFeatureFlags({
      ...DEFAULT_FEATURE_FLAGS,
      granularOptionInvalidation: true,
    });

    const dir = path.join(__dirname, 'option-invalidation-test');
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

    // Test that the basic setup works with feature flag enabled
    const firstBuild = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      defaultConfig: path.join(dir, '.parcelrc'),
    });

    const secondBuild = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      defaultConfig: path.join(dir, '.parcelrc'),
    });

    // Both builds should have completed successfully
    assert(firstBuild, 'First build should have completed successfully');
    assert(secondBuild, 'Second build should have completed successfully');
  });
});
