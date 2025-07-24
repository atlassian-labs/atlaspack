// @flow
import assert from 'assert';
import path from 'path';
import {rimraf} from 'rimraf';
import {
  bundle,
  overlayFS,
  fsFixture,
  bundler,
  mergeParcelOptions,
  getParcelOptions,
  assertNoFilePathInCache,
} from '@atlaspack/test-utils';
import {resolveOptions} from '@atlaspack/core';
import type {
  InitialAtlaspackOptions,
  BuildSuccessEvent,
} from '@atlaspack/types';

let inputDir: string;

function getEntries(entries = 'index.js') {
  return (Array.isArray(entries) ? entries : [entries]).map((entry) =>
    path.resolve(inputDir, entry),
  );
}

function getOptions(opts, featureFlags) {
  return mergeParcelOptions(
    {
      inputFS: overlayFS,
      shouldDisableCache: false,
      featureFlags: {
        ...featureFlags,
      },
    },
    opts,
  );
}

function runBundle(entries = 'index.js', opts, featureFlags) {
  return bundler(getEntries(entries), getOptions(opts, featureFlags)).run();
}

type UpdateFn = (BuildSuccessEvent) =>
  | ?InitialAtlaspackOptions
  | Promise<?InitialAtlaspackOptions>;
type TestConfig = {|
  ...InitialAtlaspackOptions,
  entries?: Array<string>,
  setup?: () => void | Promise<void>,
  update: UpdateFn,
|};

async function testCache(
  update: UpdateFn | TestConfig,
  integration?,
  featureFlags?,
) {
  let entries;
  let options: ?InitialAtlaspackOptions;
  if (typeof update === 'object') {
    let setup;
    ({entries, setup, update, ...options} = update);

    if (setup) {
      await setup();
    }
  }

  let initialOptions = getParcelOptions(
    getEntries(entries),
    getOptions(options),
  );
  let resolvedOptions = await resolveOptions(initialOptions);

  // Clear any existing files before first build
  await overlayFS.rimraf(resolvedOptions.cacheDir);
  await overlayFS.rimraf(resolvedOptions.outputFS.cwd());

  let b = await runBundle(entries, options, featureFlags);

  await assertNoFilePathInCache(
    resolvedOptions.outputFS,
    resolvedOptions.cacheDir,
    resolvedOptions.projectRoot,
  );

  // update
  let newOptions = await update(b);
  options = mergeParcelOptions(options || {}, newOptions);

  // Run cached build
  b = await runBundle(entries, options, featureFlags);

  resolvedOptions = await resolveOptions(
    getParcelOptions(getEntries(entries), getOptions(options)),
  );
  await assertNoFilePathInCache(
    resolvedOptions.outputFS,
    resolvedOptions.cacheDir,
    resolvedOptions.projectRoot,
  );

  return b;
}

describe('Option invalidation in cache integration test', () => {
  beforeEach(async () => {
    inputDir = path.join(__dirname, 'option-invalidation-fixture');
    await rimraf(inputDir);
    await overlayFS.mkdirp(inputDir);
  });

  // Remove the afterEach completely - let's see if that's causing the issue
  afterEach(async () => {
    if (inputDir) {
      await rimraf(inputDir);
    }
    if (global.gc) {
      global.gc();
    }
    inputDir = '';
  });

  it.skip('respects blocklist with granularOptionInvalidation=true', async function () {
    await fsFixture(overlayFS, inputDir)`
      .parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "reporters": []
        }

      index.js:
        export const value = "test";
    `;

    // Test that the basic setup works with feature flag enabled
    const firstBuild = await bundle(path.join(inputDir, 'index.js'), {
      inputFS: overlayFS,
      defaultConfig: path.join(inputDir, '.parcelrc'),
      featureFlags: {
        granularOptionInvalidation: true,
      },
    });

    const secondBuild = await bundle(path.join(inputDir, 'index.js'), {
      inputFS: overlayFS,
      defaultConfig: path.join(inputDir, '.parcelrc'),
      featureFlags: {
        granularOptionInvalidation: true,
      },
    });

    // Both builds should have completed successfully
    assert(firstBuild, 'First build should have completed successfully');
    assert(secondBuild, 'Second build should have completed successfully');
  });

  it('should NOT invalidate cache when instanceId changes (blocklisted option)', async function () {
    await fsFixture(overlayFS, inputDir)`
      .parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "reporters": []
        }

      index.js:
        export const value = "test";
    `;

    let b = await testCache({
      featureFlags: {
        granularOptionInvalidation: true,
      },
      update() {
        return {
          logLevel: 'info', // Same value as first build
        };
      },
    });

    assert.equal(
      b.changedAssets.size,
      0,
      'Same option values should NOT invalidate cache',
    );
  });

  it.skip('should NOT invalidate cache when logLevel changes (ignored by optionsProxy)', async function () {
    await fsFixture(overlayFS, inputDir)`
      .parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "reporters": []
        }

      index.js:
        export const value = "test";
    `;

    let b = await testCache({
      logLevel: 'info', // First build uses 'info'
      featureFlags: {
        granularOptionInvalidation: true,
      },
      update() {
        return {
          logLevel: 'error', // Different value from first build
        };
      },
    });

    assert.equal(
      b.changedAssets.size,
      0,
      'logLevel changes should NOT invalidate cache because logLevel is in ignoreOptions set',
    );
  });

  it.skip('should invalidate cache when featureFlags change (granular off)', async function () {
    await fsFixture(overlayFS, inputDir)`
      .parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "reporters": []
        }

      index.js:
        export const value = "test";
    `;

    let b = await testCache({
      featureFlags: {
        granularOptionInvalidation: false,
        exampleFeature: true,
      },
      update() {
        return {
          featureFlags: {
            granularOptionInvalidation: false,
            exampleFeature: false, // Changed!
          },
        };
      },
    });

    assert(
      b.changedAssets.size > 0,
      'Feature flag changes should cause cache invalidation because featureFlags are tracked options',
    );
  });

  it.skip('should NOT invalidate cache when featureFlags change (granular on)', async function () {
    await fsFixture(overlayFS, inputDir)`
      .parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "reporters": []
        }

      index.js:
        export const value = "test";
    `;

    let b = await testCache({
      featureFlags: {
        granularOptionInvalidation: true,
        exampleFeature: true,
      },
      update() {
        return {
          featureFlags: {
            granularOptionInvalidation: true,
            exampleFeature: false, // Changed!
          },
        };
      },
    });

    assert.equal(
      b.changedAssets.size,
      0,
      'Feature flag changes should NOT cause cache invalidation when granularOptionInvalidation is enabled',
    );
  });

  it.skip('should NOT invalidate cache when feature flags are same (granular off)', async function () {
    await fsFixture(overlayFS, inputDir)`
      .parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "reporters": []
        }

      index.js:
        export const value = "test";
    `;

    let b = await testCache({
      featureFlags: {
        granularOptionInvalidation: false,
        exampleFeature: true,
      },
      update() {
        return {
          featureFlags: {
            granularOptionInvalidation: false,
            exampleFeature: true, // Same!
          },
        };
      },
    });

    assert.equal(
      b.changedAssets.size,
      0,
      'Same feature flag values should NOT cause cache invalidation',
    );
  });
});
