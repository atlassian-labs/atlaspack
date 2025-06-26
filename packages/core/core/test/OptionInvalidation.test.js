// @flow strict-local
/* eslint-disable flowtype/no-flow-fix-me-comments */

import assert from 'assert';
import sinon from 'sinon';
import path from 'path';
import {MemoryFS} from '@atlaspack/fs';
import RequestTracker from '../src/RequestTracker';
import {RequestGraph} from '../src/RequestTracker';
import {setFeatureFlags, getFeatureFlag} from '@atlaspack/feature-flags';
import {getValueAtPath} from '../src/requests/ConfigRequest';
// $FlowFixMe - Dynamic import for WorkerFarm
const {WorkerFarm} = require('@atlaspack/workers');

// This file contains tests for the option invalidation functionality
// Note: Some Flow errors are expected due to dynamic proxies and are marked with $FlowFixMe

describe('Option Invalidation', () => {
  const projectRoot = '/project';
  const farm = new WorkerFarm({
    workerPath: require.resolve('../src/worker'),
    maxConcurrentWorkers: 1,
  });
  let fs;
  let options;
  let graph;
  let originalFeatureFlags;

  beforeEach(() => {
    // Save original feature flag values
    originalFeatureFlags = {
      enableOptionInvalidationBlocklist: getFeatureFlag(
        'enableOptionInvalidationBlocklist',
      ),
      granularOptionInvalidation: getFeatureFlag('granularOptionInvalidation'),
    };

    fs = new MemoryFS(farm);
    options = {
      inputFS: fs,
      outputFS: fs,
      workerFarm: farm,
      packageManager: null,
      projectRoot,
      inputDir: projectRoot,
      shouldDisableCache: false,
      cacheDir: path.join(projectRoot, '.cache'),
      env: {},
      targets: {},
      featureFlags: {
        enableOptionInvalidationBlocklist: false,
        granularOptionInvalidation: false,
      },
      mode: 'development',
      hot: false,
      serve: false,
      defaultConfig: {
        extends: '@atlaspack/config-default',
      },
      additionalReporters: [],
      instanceId: 'test-instance-id',
      defaultTargetOptions: {
        shouldOptimize: false,
        sourceMaps: true,
        publicUrl: '/',
      },
    };

    graph = new RequestGraph();

    // Set feature flags for the test
    setFeatureFlags(options.featureFlags);
  });

  afterEach(() => {
    // Restore original feature flag values
    setFeatureFlags({
      ...originalFeatureFlags,
    });
    farm.end();
  });

  it('tracks options that are accessed via RequestTracker API', async () => {
    const tracker = new RequestTracker({
      graph,
      farm,
      options,
    });

    // Create a mock request
    const request = {
      id: 'test_request',
      type: 0, // asset_request
      input: {id: 'test-input'},
      run: sinon.spy(({api}) => {
        api.invalidateOnOptionChange('mode');
        api.invalidateOnOptionChange(['defaultTargetOptions', 'sourceMaps']);
        api.invalidateOnOptionChange('instanceId');
        return {type: 'ok'};
      }),
    };

    // Run the request
    await tracker.runRequest(request);

    // Check that option nodes were created
    const optionNodes = graph.getNodesByPrefix('option:');
    assert.equal(optionNodes.length, 3);

    // Verify the specific option nodes exist
    const optionKeys = optionNodes.map((node) =>
      node.id.slice('option:'.length),
    );
    assert(optionKeys.includes('mode'));
    assert(optionKeys.includes('defaultTargetOptions.sourceMaps'));
    assert(optionKeys.includes('instanceId'));
  });

  it('invalidates nodes when tracked options change', async () => {
    const tracker = new RequestTracker({
      graph,
      farm,
      options,
    });

    // Create a mock request
    const request = {
      id: 'test_request',
      type: 0, // asset_request
      input: {id: 'test-input'},
      run: sinon.spy(({api}) => {
        api.invalidateOnOptionChange('mode');
        return {type: 'ok'};
      }),
    };

    // Run the request
    await tracker.runRequest(request);

    // Modify the options and check for invalidation
    const modifiedOptions = {
      ...options,
      mode: 'production', // Changed from 'development'
    };

    // Run invalidation and get the results
    const invalidatedOptions = graph.invalidateOptionNodes(modifiedOptions);

    // Check that the 'mode' option was invalidated
    assert.equal(invalidatedOptions.length, 1);
    assert.equal(invalidatedOptions[0].option, 'mode');
    assert(invalidatedOptions[0].count > 0);

    // Verify the node was actually marked as invalid
    assert(graph.invalidNodeIds.size > 0);
  });

  it('respects the blocklist when feature flag is enabled', async () => {
    // Enable the blocklist feature flag
    setFeatureFlags({
      ...options.featureFlags,
      enableOptionInvalidationBlocklist: true,
    });

    // Add instanceId to the blocklist explicitly
    options.optionInvalidation = {
      blocklist: ['instanceId'],
      useBlocklist: true,
    };

    const tracker = new RequestTracker({
      graph,
      farm,
      options,
    });

    // Create a mock request
    const request = {
      id: 'test_request',
      type: 0, // asset_request
      input: {id: 'test-input'},
      run: sinon.spy(({api}) => {
        api.invalidateOnOptionChange('mode');
        api.invalidateOnOptionChange('instanceId'); // This should be blocked
        return {type: 'ok'};
      }),
    };

    // Run the request
    await tracker.runRequest(request);

    // Modify both options
    const modifiedOptions = {
      ...options,
      mode: 'production', // Changed from 'development'
      instanceId: 'new-instance-id', // Changed but should be ignored
    };

    // Run invalidation and get the results
    const invalidatedOptions = graph.invalidateOptionNodes(modifiedOptions);

    // Check that only the 'mode' option was invalidated
    assert.equal(invalidatedOptions.length, 1);
    assert.equal(invalidatedOptions[0].option, 'mode');

    // instanceId should not cause invalidation since it's blocklisted
    assert(!invalidatedOptions.some((item) => item.option === 'instanceId'));
  });

  it('supports granular path tracking when feature flag is enabled', async () => {
    // Enable the granular option invalidation feature flag
    setFeatureFlags({
      ...options.featureFlags,
      granularOptionInvalidation: true,
    });

    options.optionInvalidation = {
      useGranularPaths: true,
    };

    // Create complex nested options for testing
    options.nestedOptions = {
      level1: {
        level2: {
          setting1: 'value1',
          setting2: 'value2',
        },
        otherSetting: true,
      },
    };

    const tracker = new RequestTracker({
      graph,
      farm,
      options,
    });

    // Create a mock request
    const request = {
      id: 'test_request',
      type: 0, // asset_request
      input: {id: 'test-input'},
      run: sinon.spy(({api}) => {
        // Track specific nested paths
        api.invalidateOnOptionChange([
          'nestedOptions',
          'level1',
          'level2',
          'setting1',
        ]);
        return {type: 'ok'};
      }),
    };

    // Run the request
    await tracker.runRequest(request);

    // Change only the specific tracked nested property
    const modifiedOptions = {
      ...options,
      nestedOptions: {
        ...options.nestedOptions,
        level1: {
          ...options.nestedOptions.level1,
          level2: {
            ...options.nestedOptions.level1.level2,
            setting1: 'changed-value', // Only this specific path is changed
          },
        },
      },
    };

    // Run invalidation and get the results
    const invalidatedOptions = graph.invalidateOptionNodes(modifiedOptions);

    // Check that the specific nested path was invalidated
    assert.equal(invalidatedOptions.length, 1);
    assert.equal(
      invalidatedOptions[0].option,
      'nestedOptions.level1.level2.setting1',
    );
  });

  it('cleans up excess option nodes to prevent memory bloat', async () => {
    const tracker = new RequestTracker({
      graph,
      farm,
      options,
    });

    // Generate many option invalidations to test cleanup
    for (let i = 0; i < 100; i++) {
      const requestId = `test_request_${i}`;
      const request = {
        id: requestId,
        type: 0, // asset_request
        input: {id: `test-input-${i}`},
        run: sinon.spy(({api}) => {
          // Create a unique option for each request
          api.invalidateOnOptionChange(`testOption${i}`);
          return {type: 'ok'};
        }),
      };

      await tracker.runRequest(request);
    }

    // Verify option nodes count
    const initialCount = graph.optionNodeIds.size;
    assert.equal(initialCount, 100);

    // Run cleanup with low threshold
    const removedCount = graph.cleanupExcessOptionNodes(50);

    // Verify that nodes were removed
    assert(removedCount > 0);
    assert(graph.optionNodeIds.size <= 50);
  });

  it('supports both string and array path formats for backward compatibility', async () => {
    const tracker = new RequestTracker({
      graph,
      farm,
      options,
    });

    // Create a mock request
    const request = {
      id: 'test_request',
      type: 0, // asset_request
      input: {id: 'test-input'},
      run: sinon.spy(({api}) => {
        // Test both formats
        api.invalidateOnOptionChange('mode'); // String format
        api.invalidateOnOptionChange(['defaultTargetOptions', 'sourceMaps']); // Array format
        return {type: 'ok'};
      }),
    };

    // Run the request
    await tracker.runRequest(request);

    // Check that option nodes were created with both formats
    const optionNodes = graph.getNodesByPrefix('option:');
    assert.equal(optionNodes.length, 2);

    const optionKeys = optionNodes.map((node) =>
      node.id.slice('option:'.length),
    );
    assert(optionKeys.includes('mode'));
    assert(optionKeys.includes('defaultTargetOptions.sourceMaps'));
  });

  it('getValueAtPath correctly navigates nested paths', () => {
    const testObj = {
      a: {
        b: {
          c: 'value',
          d: [1, 2, 3],
        },
      },
      x: null,
      y: undefined,
    };

    // Test array path access
    assert.equal(getValueAtPath(testObj, ['a', 'b', 'c']), 'value');
    assert.deepEqual(getValueAtPath(testObj, ['a', 'b', 'd']), [1, 2, 3]);

    // Test empty path returns original object
    assert.deepEqual(getValueAtPath(testObj, []), testObj);

    // Test non-existent paths
    assert.equal(getValueAtPath(testObj, ['a', 'b', 'z']), undefined);
    assert.equal(getValueAtPath(testObj, ['non', 'existent']), undefined);

    // Test null and undefined handling
    assert.equal(getValueAtPath(testObj, ['x']), null);
    assert.equal(getValueAtPath(testObj, ['x', 'something']), undefined);
    assert.equal(getValueAtPath(testObj, ['y']), undefined);
    assert.equal(getValueAtPath(testObj, ['y', 'something']), undefined);
  });
});
