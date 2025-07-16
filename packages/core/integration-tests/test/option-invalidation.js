// @flow
import assert from 'assert';
import path from 'path';
import {bundle, overlayFS, fsFixture, bundler} from '@atlaspack/test-utils';
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

  it('should NOT invalidate cache when ignored options change', async function () {
    setFeatureFlags({
      ...DEFAULT_FEATURE_FLAGS,
      granularOptionInvalidation: true,
    });

    const dir = path.join(__dirname, 'option-invalidation-test-blocklist');
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

    // First build with original options
    await bundler([path.join(dir, 'index.js')], {
      inputFS: overlayFS,
      shouldDisableCache: false,
      defaultConfig: path.join(dir, '.parcelrc'),
      logLevel: 'info',
      shouldProfile: false,
    }).run();

    // Second build with changed ignored options -- should NOT invalidate cache
    const secondBuild = await bundler([path.join(dir, 'index.js')], {
      inputFS: overlayFS,
      shouldDisableCache: false,
      defaultConfig: path.join(dir, '.parcelrc'),
      logLevel: 'error',
      shouldProfile: false,
    }).run();

    assert.equal(
      secondBuild.changedAssets.size,
      0,
      'Ignored options should not invalidate cache',
    );
  });

  it('should invalidate cache when non-blocklisted options change and granularOptionInvalidation is enabled', async function () {
    setFeatureFlags({
      ...DEFAULT_FEATURE_FLAGS,
      granularOptionInvalidation: true,
    });

    const dir = path.join(__dirname, 'option-invalidation-test-2');
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

    await bundler([path.join(dir, 'index.js')], {
      inputFS: overlayFS,
      shouldDisableCache: false,
      defaultConfig: path.join(dir, '.parcelrc'),
      mode: 'development',
    }).run();

    // Second build with production mode (should invalidate cache)
    const secondBuild = await bundler([path.join(dir, 'index.js')], {
      inputFS: overlayFS,
      shouldDisableCache: false,
      defaultConfig: path.join(dir, '.parcelrc'),
      // mode is not in the blocklist, so it should invalidate cache
      mode: 'production',
    }).run();

    assert(
      secondBuild.changedAssets.size > 0,
      'Non-blocklisted options should invalidate cache when granularOptionInvalidation is enabled',
    );
  });

  it('should invalidate cache for mode changes regardless of granularOptionInvalidation setting', async function () {
    setFeatureFlags({
      ...DEFAULT_FEATURE_FLAGS,
      granularOptionInvalidation: false,
    });

    const dir = path.join(
      __dirname,
      'option-invalidation-test-disabled-feature',
    );
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

    await bundler([path.join(dir, 'index.js')], {
      inputFS: overlayFS,
      shouldDisableCache: false,
      defaultConfig: path.join(dir, '.parcelrc'),
      mode: 'development',
    }).run();

    // Second build with production mode -- should invalidate cache
    const secondBuild = await bundler([path.join(dir, 'index.js')], {
      inputFS: overlayFS,
      shouldDisableCache: false,
      defaultConfig: path.join(dir, '.parcelrc'),
      mode: 'production', // This should cause invalidation
    }).run();

    assert(
      secondBuild.changedAssets.size > 0,
      'Mode changes should always invalidate cache regardless of granularOptionInvalidation setting',
    );
  });
});
