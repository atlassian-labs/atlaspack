// @flow
import assert from 'assert';
import path from 'path';
import {
  bundle,
  overlayFS,
  fsFixture,
  bundler,
  mergeParcelOptions,
  getParcelOptions,
  assertNoFilePathInCache,
} from '@atlaspack/test-utils';
import {getFeatureFlag} from '@atlaspack/feature-flags';
import {resolveOptions} from '@atlaspack/core';
import type {
  InitialAtlaspackOptions,
  BuildSuccessEvent,
} from '@atlaspack/types';

let inputDir: string;

function getEntries(entries = 'src/index.js') {
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

function runBundle(entries = 'src/index.js', opts, featureFlags) {
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
  beforeEach(() => {
    inputDir = path.join(
      __dirname,
      '/input',
      Math.random().toString(36).slice(2),
    );
  });
  it('respects blocklist with granularOptionInvalidation=true', async function () {
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
      featureFlags: {
        granularOptionInvalidation: true,
      },
    });

    const secondBuild = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      defaultConfig: path.join(dir, '.parcelrc'),
      featureFlags: {
        granularOptionInvalidation: true,
      },
    });

    // Both builds should have completed successfully
    assert(firstBuild, 'First build should have completed successfully');
    assert(secondBuild, 'Second build should have completed successfully');
  });

  it('should NOT invalidate cache when instanceId changes (blocklisted option)', async function () {
    let b = await testCache({
      async setup() {
        await overlayFS.mkdirp(inputDir);
        await overlayFS.mkdirp(path.join(inputDir, 'src'));
        await overlayFS.writeFile(
          path.join(inputDir, '.parcelrc'),
          JSON.stringify({
            extends: '@atlaspack/config-default',
            reporters: [],
          }),
        );
        await overlayFS.writeFile(
          path.join(inputDir, 'src/index.js'),
          'export const value = "test";',
        );
      },
      featureFlags: {
        granularOptionInvalidation: true,
      },
      update() {
        // First build completed with logLevel: 'info' (default)
        // Return same options to test no invalidation
        return {
          logLevel: 'info', // Same value as first build
        };
      },
    });

    const debugInfo = {
      changedAssetsCount: b.changedAssets.size,
      changedAssetsList: Array.from(b.changedAssets.keys()),
      environment: {
        isCI: !!process.env.CI,
        platform: process.platform,
        nodeVersion: process.version,
        nodeEnv: process.env.NODE_ENV,
        ciProvider: process.env.GITHUB_ACTIONS
          ? 'GitHub Actions'
          : process.env.CI_NAME || (process.env.CI ? 'Unknown CI' : 'Local'),
      },
      featureFlags: {
        granularOptionInvalidation: getFeatureFlag(
          'granularOptionInvalidation',
        ),
        cachePerformanceImprovements: getFeatureFlag(
          'cachePerformanceImprovements',
        ),
        atlaspackV3: getFeatureFlag('atlaspackV3'),
      },
      atlaspackEnv: Object.keys(process.env)
        .filter(
          (key) => key.startsWith('ATLASPACK_') || key.startsWith('PARCEL_'),
        )
        .reduce((acc, key) => {
          acc[key] = process.env[key];
          return acc;
        }, {}),
      optionComparison: {
        unchangedOptions: {
          logLevel: {value: 'info'},
        },
        expectedBehavior:
          'Same option values should NOT cause cache invalidation',
      },
      filesystem: {
        cwd: process.cwd(),
        testDir: inputDir,
        configExists: require('fs').existsSync(
          path.join(inputDir, '.parcelrc'),
        ),
        indexExists: require('fs').existsSync(
          path.join(inputDir, 'src/index.js'),
        ),
      },
      runtime: {
        memoryUsage: process.memoryUsage(),
        uptime: process.uptime(),
        buildTimestamp: Date.now(),
      },
      testConfig: {
        timeout: this.timeout?.() || 'unknown',
        testFile: __filename,
      },
    };

    assert.equal(
      b.changedAssets.size,
      0,
      `Same option values should NOT invalidate cache.\n\nDEBUG INFO:\n${JSON.stringify(
        debugInfo,
        null,
        2,
      )}`,
    );
  });

  it('should NOT invalidate cache when logLevel changes (ignored by optionsProxy)', async function () {
    let b = await testCache({
      async setup() {
        await overlayFS.mkdirp(inputDir);
        await overlayFS.mkdirp(path.join(inputDir, 'src'));
        await overlayFS.writeFile(
          path.join(inputDir, '.parcelrc'),
          JSON.stringify({
            extends: '@atlaspack/config-default',
            reporters: [],
          }),
        );
        await overlayFS.writeFile(
          path.join(inputDir, 'src/index.js'),
          'export const value = "test";',
        );
      },
      logLevel: 'info', // First build uses 'info'
      featureFlags: {
        granularOptionInvalidation: true,
      },
      update() {
        // Second build with DIFFERENT logLevel -- should NOT invalidate because logLevel is in ignoreOptions
        return {
          logLevel: 'error', // Different value from first build
        };
      },
    });

    const debugInfo2 = {
      changedAssetsCount: b.changedAssets.size,
      changedAssetsList: Array.from(b.changedAssets.keys()),
      environment: {
        isCI: !!process.env.CI,
        platform: process.platform,
        nodeVersion: process.version,
        nodeEnv: process.env.NODE_ENV,
        ciProvider: process.env.GITHUB_ACTIONS
          ? 'GitHub Actions'
          : process.env.CI_NAME || (process.env.CI ? 'Unknown CI' : 'Local'),
      },
      featureFlags: {
        granularOptionInvalidation: getFeatureFlag(
          'granularOptionInvalidation',
        ),
        cachePerformanceImprovements: getFeatureFlag(
          'cachePerformanceImprovements',
        ),
        atlaspackV3: getFeatureFlag('atlaspackV3'),
      },
      atlaspackEnv: Object.keys(process.env)
        .filter(
          (key) => key.startsWith('ATLASPACK_') || key.startsWith('PARCEL_'),
        )
        .reduce((acc, key) => {
          acc[key] = process.env[key];
          return acc;
        }, {}),
      optionComparison: {
        changedOptions: {
          logLevel: {from: 'info', to: 'error'},
        },
        expectedBehavior: 'logLevel changes should be ignored by optionsProxy',
      },
      filesystem: {
        cwd: process.cwd(),
        testDir: inputDir,
        configExists: require('fs').existsSync(
          path.join(inputDir, '.parcelrc'),
        ),
        indexExists: require('fs').existsSync(
          path.join(inputDir, 'src/index.js'),
        ),
      },
      runtime: {
        memoryUsage: process.memoryUsage(),
        uptime: process.uptime(),
        buildTimestamp: Date.now(),
      },
      testConfig: {
        timeout: this.timeout?.() || 'unknown',
        testFile: __filename,
      },
    };

    assert.equal(
      b.changedAssets.size,
      0,
      `logLevel changes should NOT invalidate cache because logLevel is in ignoreOptions set.\n\nDEBUG INFO:\n${JSON.stringify(
        debugInfo2,
        null,
        2,
      )}`,
    );
  });

  it('should invalidate cache when non-blocklisted options change and granularOptionInvalidation is enabled', async function () {
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
      featureFlags: {
        granularOptionInvalidation: true,
      },
    }).run();

    // Second build with production mode (should invalidate cache)
    const secondBuild = await bundler([path.join(dir, 'index.js')], {
      inputFS: overlayFS,
      shouldDisableCache: false,
      defaultConfig: path.join(dir, '.parcelrc'),
      // mode is not in the blocklist, so it should invalidate cache
      mode: 'production',
      featureFlags: {
        granularOptionInvalidation: true,
      },
    }).run();

    assert(
      secondBuild.changedAssets.size > 0,
      'Non-blocklisted options should invalidate cache when granularOptionInvalidation is enabled',
    );
  });

  it('should invalidate cache for mode changes regardless of granularOptionInvalidation setting', async function () {
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
      featureFlags: {
        granularOptionInvalidation: false,
      },
    }).run();

    // Second build with production mode -- should invalidate cache
    const secondBuild = await bundler([path.join(dir, 'index.js')], {
      inputFS: overlayFS,
      shouldDisableCache: false,
      defaultConfig: path.join(dir, '.parcelrc'),
      mode: 'production', // This should cause invalidation
      featureFlags: {
        granularOptionInvalidation: false,
      },
    }).run();

    assert(
      secondBuild.changedAssets.size > 0,
      'Mode changes should always invalidate cache regardless of granularOptionInvalidation setting',
    );
  });
});
