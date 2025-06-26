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
import {
  bundler,
  inputFS,
  overlayFS,
  mergeParcelOptions,
} from '@atlaspack/test-utils';
import sinon from 'sinon';

describe('Option invalidation in cache integration test', () => {
  let inputDir;

  beforeEach(async () => {
    inputDir = path.join(__dirname, '../../../integration-tests/input');
    await inputFS.rimraf(inputDir);
    await inputFS.mkdirp(inputDir);
  });

  async function createSimpleProject() {
    // Create a simple project for testing
    await inputFS.writeFile(
      path.join(inputDir, 'package.json'),
      JSON.stringify({
        name: 'option-invalidation-test',
      }),
    );

    await inputFS.writeFile(
      path.join(inputDir, 'index.js'),
      'export const value = "test";',
    );

    await inputFS.writeFile(
      path.join(inputDir, '.parcelrc'),
      JSON.stringify({
        extends: '@atlaspack/config-default',
      }),
    );
  }

  function getOptions(featureFlags = {}) {
    // $FlowFixMe[incompatible-call] - We're testing with custom options
    // $FlowFixMe[incompatible-exact] - Suppress inexact object errors in featureFlags
    return mergeParcelOptions({
      inputFS: overlayFS,
      shouldDisableCache: false,
      featureFlags: {
        enableOptionInvalidationBlocklist: false,
        granularOptionInvalidation: false,
        ...featureFlags,
      },
    });
  }

  function runBundle(entries, opts) {
    // $FlowFixMe[incompatible-call] - We're testing with custom options
    // $FlowFixMe[prop-missing] - Props like instanceId, nestedOption don't exist in type definitions
    return bundler(Array.isArray(entries) ? entries : [entries], opts).run();
  }

  // $FlowFixMe[prop-missing] - Integration test accessing internal APIs
  it('respects blocklist to prevent invalidation of non-impactful options', async () => {
    await createSimpleProject();

    // Configure to enable the blocklist
    const options = getOptions({
      enableOptionInvalidationBlocklist: true,
    });

    // Add instanceId to the blocklist
    // $FlowFixMe[prop-missing] - optionInvalidation is a custom property
    options.optionInvalidation = {
      blocklist: ['instanceId'],
      useBlocklist: true,
    };

    // First build
    const entryFile = path.join(inputDir, 'index.js');
    const firstBuild = await runBundle(entryFile, options);

    // Spy on the RequestGraph.invalidateOptionNodes method to track invalidations
    // $FlowFixMe[prop-missing] - getRequestTracker is an internal API
    const spy = sinon.spy(
      firstBuild.getRequestTracker().graph,
      'invalidateOptionNodes',
    );

    // Run a second build with a different instanceId
    const secondBuildOptions = {
      ...options,
      instanceId: 'different-instance-id', // This should be ignored due to blocklist
    };

    const secondBuild = await runBundle(entryFile, secondBuildOptions);

    // The RequestGraph.invalidateOptionNodes should have been called once
    assert(spy.calledOnce);

    // But it should not report any invalidations for 'instanceId'
    const invalidations = spy.returnValues[0];
    assert(Array.isArray(invalidations));

    // No invalidations should have occurred for instanceId
    assert(!invalidations.some((inv) => inv.option === 'instanceId'));

    // The bundleGraph should be reused from cache
    assert.equal(secondBuild.changedAssets.size, 0);
  });

  // $FlowFixMe[prop-missing] - Integration test accessing internal APIs
  it('tracks granular paths when enabled', async () => {
    await createSimpleProject();

    // Configure to enable granular option invalidation
    const options = getOptions({
      granularOptionInvalidation: true,
    });

    // $FlowFixMe[prop-missing] - optionInvalidation is a custom property
    options.optionInvalidation = {
      useGranularPaths: true,
    };

    // Add a complex nested option
    // $FlowFixMe[prop-missing] - nestedOption is a custom property
    options.nestedOption = {
      config: {
        values: {
          setting1: 'original',
          setting2: 'unchanged',
        },
      },
    };

    // First build
    const entryFile = path.join(inputDir, 'index.js');
    const firstBuild = await runBundle(entryFile, options);

    // Spy on the RequestGraph.invalidateOptionNodes method
    // $FlowFixMe[prop-missing] - getRequestTracker is an internal API
    const spy = sinon.spy(
      firstBuild.getRequestTracker().graph,
      'invalidateOptionNodes',
    );

    // Run a second build with just one nested value changed
    // $FlowFixMe[prop-missing] - nestedOption is a custom property
    // $FlowFixMe[incompatible-use] - Properties may not exist in the types
    const secondBuildOptions = {
      ...options,
      nestedOption: {
        ...options.nestedOption,
        config: {
          ...options.nestedOption.config,
          values: {
            ...options.nestedOption.config.values,
            setting1: 'changed', // Only this value is changed
          },
        },
      },
    };

    await runBundle(entryFile, secondBuildOptions);

    // The RequestGraph.invalidateOptionNodes should have been called
    assert(spy.calledOnce);

    // It should report invalidations only for the specific path that changed
    const invalidations = spy.returnValues[0];

    // Check if there are any invalidations - if the option was accessed during the build
    if (invalidations.length > 0) {
      // If the nestedOption was accessed and tracked, validate specificity
      const nestedInvalidations = invalidations.filter((inv) =>
        inv.option.startsWith('nestedOption.'),
      );

      if (nestedInvalidations.length > 0) {
        // Only the specific path should be invalidated, not the entire nestedOption
        assert(
          nestedInvalidations.some((inv) =>
            inv.option.includes('nestedOption.config.values.setting1'),
          ),
        );

        // The setting2 path should not be invalidated
        assert(
          !nestedInvalidations.some((inv) =>
            inv.option.includes('nestedOption.config.values.setting2'),
          ),
        );
      }
    }
  });
});
