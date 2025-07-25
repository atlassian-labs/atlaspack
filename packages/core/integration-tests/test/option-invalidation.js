// @flow
import assert from 'assert';
import path from 'path';

import {
  overlayFS,
  bundler,
  mergeParcelOptions,
  getParcelOptions,
  ncp,
  assertNoFilePathInCache,
  run,
} from '@atlaspack/test-utils';
import {resolveOptions} from '@atlaspack/core';
import type {
  InitialAtlaspackOptions,
  BuildSuccessEvent,
} from '@atlaspack/types';
import type {FeatureFlags} from '@atlaspack/feature-flags';

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
  integration,
  featureFlags?: $Shape<FeatureFlags>,
) {
  await ncp(
    path.join(__dirname, '/integration', integration ?? 'cache'),
    path.join(inputDir),
  );

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

  it('should invalidate cache when shouldContentHash changes from false to true', async function () {
    let b = await testCache({
      entries: ['src/index.html'],
      defaultTargetOptions: {
        shouldScopeHoist: true,
      },
      shouldContentHash: false,
      update(b) {
        // Check that bundle path doesn't include hash when shouldContentHash is false
        let bundle = b.bundleGraph.getBundles()[1];
        assert(
          bundle.filePath.includes(bundle.id.slice(-8)),
          'should include hash in path',
        );

        return {
          shouldContentHash: true,
        };
      },
    });

    // Check that bundle path includes hash when shouldContentHash is true
    let bundle = b.bundleGraph.getBundles()[1];
    assert(
      !bundle.filePath.includes(bundle.id.slice(-8)),
      'should not include hash in path',
    );
  });

  it('should NOT invalidate cache when logLevel changes from info to error', async function () {
    let b = await testCache({
      logLevel: 'info',
      async update(b) {
        assert.equal(await run(b.bundleGraph), 4);
        return {
          logLevel: 'error',
        };
      },
    });

    // Should still get the same result because logLevel is ignored
    assert.equal(await run(b.bundleGraph), 4);
    assert.equal(b.changedAssets.size, 0, 'Cache should not be invalidated');
  });

  it('should invalidate cache when featureFlags change (granularOptionInvalidation off)', async function () {
    let b = await testCache({
      featureFlags: {
        granularOptionInvalidation: false,
        exampleFeature: true,
      },
      async update(b) {
        assert.equal(await run(b.bundleGraph), 4);
        return {
          featureFlags: {
            granularOptionInvalidation: false,
            exampleFeature: false, // Changed!
          },
        };
      },
    });

    // Should get the same result but with cache invalidation
    assert.equal(await run(b.bundleGraph), 4);
    assert(b.changedAssets.size > 0, 'Cache should be invalidated');
  });

  it('should NOT invalidate cache when featureFlags change (granularOptionInvalidation on)', async function () {
    let b = await testCache({
      featureFlags: {
        granularOptionInvalidation: true,
        exampleFeature: true,
      },
      async update(b) {
        assert.equal(await run(b.bundleGraph), 4);
        return {
          featureFlags: {
            granularOptionInvalidation: true,
            exampleFeature: false, // Changed!
          },
        };
      },
    });

    // Should still get the same result because granular invalidation is on
    assert.equal(await run(b.bundleGraph), 4);
    assert.equal(b.changedAssets.size, 0, 'Cache should not be invalidated');
  });
});
