// @flow strict-local

import assert from 'assert';
import nullthrows from 'nullthrows';
import RequestTracker, {
  type RunAPI,
  cleanUpOrphans,
  runInvalidation,
  getBiggestFSEventsInvalidations,
  invalidateRequestGraphFSEvents,
  loadEnvironmentsFromCache,
  storeEnvById,
  storeEnvManager,
  writeEnvironments,
} from '../src/RequestTracker';
import {Graph} from '@atlaspack/graph';
import {LMDBLiteCache} from '@atlaspack/cache';
import WorkerFarm from '@atlaspack/workers';
import {DEFAULT_OPTIONS} from './test-utils';
import {
  FILE_CREATE,
  FILE_UPDATE,
  INITIAL_BUILD,
  ATLASPACK_VERSION,
} from '../src/constants';
import {makeDeferredWithPromise} from '@atlaspack/utils';
import {toProjectPath} from '../src/projectPath';
import {DEFAULT_FEATURE_FLAGS, setFeatureFlags} from '../../feature-flags/src';
import sinon from 'sinon';
import type {AtlaspackOptions} from '../src/types';
import {setAllEnvironments, getAllEnvironments} from '@atlaspack/rust';

const options = {
  ...DEFAULT_OPTIONS,
  cache: new LMDBLiteCache(DEFAULT_OPTIONS.cacheDir),
};
const farm = new WorkerFarm({workerPath: require.resolve('../src/worker')});

describe('RequestTracker', () => {
  beforeEach(async () => {
    await options.cache.ensure();

    for (const key of options.cache.keys()) {
      await options.cache.getNativeRef().delete(key);
    }
  });

  it('should not run requests that have not been invalidated', async () => {
    let tracker = new RequestTracker({farm, options});
    await tracker.runRequest({
      id: 'abc',
      type: 7,
      run: () => {},
      input: null,
    });
    let called = false;
    await tracker.runRequest({
      id: 'abc',
      type: 7,
      run: () => {
        called = true;
      },
      input: null,
    });
    assert(called === false);
  });

  it('should rerun requests that have been invalidated', async () => {
    let tracker = new RequestTracker({farm, options});
    await tracker.runRequest({
      id: 'abc',
      type: 7,
      run: () => {},
      input: null,
    });
    tracker.graph.invalidateNode(
      tracker.graph.getNodeIdByContentKey('abc'),
      INITIAL_BUILD,
    );
    let called = false;
    await tracker.runRequest({
      id: 'abc',
      type: 7,
      run: () => {
        called = true;
      },
      input: null,
    });
    assert(called === true);
  });

  it('should invalidate requests with invalidated subrequests', async () => {
    let tracker = new RequestTracker({farm, options});
    await tracker.runRequest({
      id: 'abc',
      type: 7,
      run: async ({api}) => {
        await api.runRequest({
          id: 'xyz',
          type: 7,
          run: () => {},
          input: null,
        });
      },
      input: null,
    });
    tracker.graph.invalidateNode(
      tracker.graph.getNodeIdByContentKey('xyz'),
      INITIAL_BUILD,
    );
    assert(
      tracker
        .getInvalidRequests()
        .map((req) => req.id)
        .includes('abc'),
    );
  });

  it('should invalidate requests that failed', async () => {
    let tracker = new RequestTracker({farm, options});
    await tracker
      .runRequest({
        id: 'abc',
        type: 7,
        run: async () => {
          await Promise.resolve();
          throw new Error('woops');
        },
        input: null,
      })
      .then(null, () => {
        /* do nothing */
      });
    assert(
      tracker
        .getInvalidRequests()
        .map((req) => req.id)
        .includes('abc'),
    );
  });

  it('should remove subrequests that are no longer called within a request', async () => {
    let tracker = new RequestTracker({farm, options});
    await tracker.runRequest({
      id: 'abc',
      type: 7,
      run: async ({api}) => {
        await api.runRequest({
          id: 'xyz',
          type: 7,
          run: () => {},
          input: null,
        });
      },
      input: null,
    });
    let nodeId = nullthrows(tracker.graph.getNodeIdByContentKey('abc'));
    tracker.graph.invalidateNode(nodeId, INITIAL_BUILD);
    await tracker.runRequest({
      id: 'abc',
      type: 7,
      run: async ({api}) => {
        await api.runRequest({
          id: '123',
          type: 7,
          run: () => {},
          input: null,
        });
      },
      input: null,
    });
    assert(!tracker.graph.hasContentKey('xyz'));
  });

  it('should return a cached result if it was stored', async () => {
    let tracker = new RequestTracker({farm, options});
    await tracker.runRequest({
      id: 'abc',
      type: 7,
      // $FlowFixMe string isn't a valid result
      run: async ({api}: {api: RunAPI<string | void>, ...}) => {
        let result = await Promise.resolve('hello');
        api.storeResult(result);
      },
      input: null,
    });
    let result = await tracker.runRequest({
      id: 'abc',
      type: 7,
      run: async () => {},
      input: null,
    });
    assert(result === 'hello');
  });

  it('should reject all in progress requests when the abort controller aborts', async () => {
    let tracker = new RequestTracker({farm, options});
    let p = tracker
      .runRequest({
        id: 'abc',
        type: 7,
        run: async () => {
          await Promise.resolve('hello');
        },
        input: null,
      })
      .then(null, () => {
        /* do nothing */
      });
    // $FlowFixMe
    tracker.setSignal({aborted: true});
    await p;
    assert(
      tracker
        .getInvalidRequests()
        .map((req) => req.id)
        .includes('abc'),
    );
  });

  it('should write cache to disk and store index', async () => {
    let tracker = new RequestTracker({farm, options});

    await tracker.runRequest({
      id: 'abc',
      type: 7,
      // $FlowFixMe string isn't a valid result
      run: async ({api}: {api: RunAPI<string | void>, ...}) => {
        let result = await Promise.resolve();
        api.storeResult(result);
      },
      input: null,
    });

    await tracker.writeToCache();

    assert(tracker.graph.cachedRequestChunks.size > 0);
  });

  it('should not write to cache when the abort controller aborts', async () => {
    let tracker = new RequestTracker({farm, options});

    const abortController = new AbortController();
    abortController.abort();

    await tracker.writeToCache(abortController.signal);

    assert(tracker.graph.cachedRequestChunks.size === 0);
  });

  it('should not requeue requests if the previous request is still running', async () => {
    let tracker = new RequestTracker({farm, options});

    let lockA = makeDeferredWithPromise();
    let lockB = makeDeferredWithPromise();

    let requestA = tracker.runRequest({
      id: 'abc',
      type: 7,
      // $FlowFixMe string isn't a valid result
      run: async ({api}: {api: RunAPI<string>, ...}) => {
        await lockA.promise;
        api.storeResult('a');
        return 'a';
      },
      input: null,
    });

    let calledB = false;
    let requestB = tracker.runRequest({
      id: 'abc',
      type: 7,
      // $FlowFixMe string isn't a valid result
      run: async ({api}: {api: RunAPI<string>, ...}) => {
        calledB = true;
        await lockB.promise;
        api.storeResult('b');
        return 'b';
      },
      input: null,
    });

    lockA.deferred.resolve();
    lockB.deferred.resolve();
    let resultA = await requestA;
    let resultB = await requestB;
    assert.strictEqual(resultA, 'a');
    assert.strictEqual(resultB, 'a');
    assert.strictEqual(calledB, false);

    let cachedResult = await tracker.runRequest({
      id: 'abc',
      type: 7,
      run: () => {},
      input: null,
    });
    assert.strictEqual(cachedResult, 'a');
  });

  it('should requeue requests if the previous request is still running but failed', async () => {
    let tracker = new RequestTracker({farm, options});

    let lockA = makeDeferredWithPromise();
    let lockB = makeDeferredWithPromise();

    let requestA = tracker
      .runRequest({
        id: 'abc',
        type: 7,
        run: async () => {
          await lockA.promise;
          throw new Error('whoops');
        },
        input: null,
      })
      .catch(() => {
        // ignore
      });

    let requestB = tracker.runRequest({
      id: 'abc',
      type: 7,
      // $FlowFixMe string isn't a valid result
      run: async ({api}: {api: RunAPI<string | void>, ...}) => {
        await lockB.promise;
        api.storeResult('b');
      },
      input: null,
    });

    lockA.deferred.resolve();
    lockB.deferred.resolve();
    await requestA;
    await requestB;

    let called = false;
    let cachedResult = await tracker.runRequest({
      id: 'abc',
      type: 7,
      run: () => {
        called = true;
      },
      input: null,
    });
    assert.strictEqual(cachedResult, 'b');
    assert.strictEqual(called, false);
  });

  it('should ignore stale node chunks from cache', async () => {
    let tracker = new RequestTracker({farm, options});

    // Set the nodes per blob low so we can ensure multiple files without
    // creating 17,000 nodes
    tracker.graph.nodesPerBlob = 2;

    tracker.graph.addNode({type: 0, id: 'some-file-node-1'});
    tracker.graph.addNode({type: 0, id: 'some-file-node-2'});
    tracker.graph.addNode({type: 0, id: 'some-file-node-3'});
    tracker.graph.addNode({type: 0, id: 'some-file-node-4'});
    tracker.graph.addNode({type: 0, id: 'some-file-node-5'});

    await tracker.writeToCache();

    // Create a new request tracker that shouldn't look at the old cache files
    tracker = new RequestTracker({farm, options});
    assert.equal(tracker.graph.nodes.length, 0);

    tracker.graph.addNode({type: 0, id: 'some-file-node-1'});
    await tracker.writeToCache();

    // Init a request tracker that should only read the relevant cache files
    tracker = await RequestTracker.init({farm, options});
    assert.equal(tracker.graph.nodes.length, 1);
  });

  it('should init with multiple node chunks', async () => {
    let tracker = new RequestTracker({farm, options});

    // Set the nodes per blob low so we can ensure multiple files without
    // creating 17,000 nodes
    tracker.graph.nodesPerBlob = 2;

    tracker.graph.addNode({type: 0, id: 'some-file-node-1'});
    tracker.graph.addNode({type: 0, id: 'some-file-node-2'});
    tracker.graph.addNode({type: 0, id: 'some-file-node-3'});
    tracker.graph.addNode({type: 0, id: 'some-file-node-4'});
    tracker.graph.addNode({type: 0, id: 'some-file-node-5'});

    await tracker.writeToCache();

    tracker = await RequestTracker.init({farm, options});
    assert.equal(tracker.graph.nodes.length, 5);
  });

  it('should write new nodes to cache', async () => {
    let tracker = new RequestTracker({farm, options});

    tracker.graph.addNode({
      type: 0,
      id: 'test-file',
    });
    await tracker.writeToCache();
    assert.equal(tracker.graph.nodes.length, 1);

    tracker.graph.addNode({
      type: 0,
      id: 'test-file-2',
    });
    await tracker.writeToCache();
    assert.equal(tracker.graph.nodes.length, 2);

    // Create a new tracker from cache
    tracker = await RequestTracker.init({farm, options});

    await tracker.writeToCache();
    assert.equal(tracker.graph.nodes.length, 2);
  });

  it('should write updated nodes to cache', async () => {
    let tracker = new RequestTracker({farm, options});

    let contentKey = 'abc';
    await tracker.runRequest({
      id: contentKey,
      type: 7,
      // $FlowFixMe string isn't a valid result
      run: async ({api}: {api: RunAPI<string | void>, ...}) => {
        let result = await Promise.resolve('a');
        api.storeResult(result);
      },
      input: null,
    });
    assert.equal(await tracker.getRequestResult(contentKey), 'a');
    await tracker.writeToCache();

    await tracker.runRequest(
      {
        id: contentKey,
        type: 7,
        // $FlowFixMe string isn't a valid result
        run: async ({api}: {api: RunAPI<string | void>, ...}) => {
          let result = await Promise.resolve('b');
          api.storeResult(result);
        },
        input: null,
      },
      {force: true},
    );
    assert.equal(await tracker.getRequestResult(contentKey), 'b');
    await tracker.writeToCache();

    // Create a new tracker from cache
    tracker = await RequestTracker.init({farm, options});

    assert.equal(await tracker.getRequestResult(contentKey), 'b');
  });

  it('should write invalidated nodes to cache', async () => {
    let tracker = new RequestTracker({farm, options});

    let contentKey = 'abc';
    await tracker.runRequest({
      id: contentKey,
      type: 7,
      run: () => {},
      input: null,
    });
    let nodeId = tracker.graph.getNodeIdByContentKey(contentKey);
    assert.equal(tracker.graph.getNode(nodeId)?.invalidateReason, 0);
    await tracker.writeToCache();

    tracker.graph.invalidateNode(nodeId, 1);
    assert.equal(tracker.graph.getNode(nodeId)?.invalidateReason, 1);
    await tracker.writeToCache();

    // Create a new tracker from cache
    tracker = await RequestTracker.init({farm, options});

    assert.equal(tracker.graph.getNode(nodeId)?.invalidateReason, 1);
  });

  describe('respondToFSEvents', () => {
    [true, false].forEach((value) => {
      beforeEach(() => {
        setFeatureFlags({
          ...DEFAULT_FEATURE_FLAGS,
          fixQuadraticCacheInvalidation: value === true ? 'NEW' : 'OLD',
        });
      });

      afterEach(() => {
        setFeatureFlags({
          ...DEFAULT_FEATURE_FLAGS,
        });
      });

      describe(`optimizations feature-flag ${String(value)}`, () => {
        it('should invalidate file requests on file changes', async () => {
          let tracker = new RequestTracker({farm, options});
          await tracker.runRequest({
            id: 'abc',
            type: 7,
            // $FlowFixMe string isn't a valid result
            run: async ({api}: {api: RunAPI<string | void>, ...}) => {
              api.invalidateOnFileUpdate(toProjectPath('', 'my-file'));
              let result = await Promise.resolve('hello');
              api.storeResult(result);
            },
            input: null,
          });
          const requestId = tracker.graph.getNodeIdByContentKey('abc');
          const {didInvalidate: invalidated} = await tracker.respondToFSEvents(
            [
              {
                type: 'update',
                path: 'my-file',
              },
            ],
            Number.MAX_VALUE,
          );
          assert.equal(invalidated, true);
          assert.equal(
            tracker.graph.getNode(requestId)?.invalidateReason,
            FILE_UPDATE,
          );
          assert.deepEqual(Array.from(tracker.graph.invalidNodeIds), [
            requestId,
          ]);
        });

        it('should invalidate file name requests with invalidate above invalidations', async () => {
          let tracker = new RequestTracker({farm, options});
          await tracker.runRequest({
            id: 'abc',
            type: 7,
            // $FlowFixMe string isn't a valid result
            run: async ({api}: {api: RunAPI<string | void>, ...}) => {
              api.invalidateOnFileCreate({
                fileName: 'package.json',
                aboveFilePath: toProjectPath(
                  '',
                  './node_modules/something/package.json',
                ),
              });
              let result = await Promise.resolve('hello');
              api.storeResult(result);
            },
            input: null,
          });
          const requestId = tracker.graph.getNodeIdByContentKey('abc');
          const {didInvalidate: invalidated} = await tracker.respondToFSEvents(
            [
              {
                type: 'create',
                path: './package.json',
              },
            ],
            Number.MAX_VALUE,
          );
          assert.equal(invalidated, true);
          assert.equal(
            tracker.graph.getNode(requestId)?.invalidateReason,
            FILE_CREATE,
          );
          assert.deepEqual(Array.from(tracker.graph.invalidNodeIds), [
            requestId,
          ]);
        });
      });
    });
  });
});

describe('cleanUpOrphans', () => {
  it('cleans-up unreachable nodes', () => {
    const graph: Graph<string, number> = new Graph();
    const root = graph.addNode('root');
    graph.setRootNodeId(root);
    const node1 = graph.addNode('node1');
    const node2 = graph.addNode('node2');
    const node3 = graph.addNode('node3');
    const orphan1 = graph.addNode('orphan1');
    const orphan2 = graph.addNode('orphan2');

    /*

root --- node1 --- node2 ----------- orphan1 --- orphan2
     \---- node3          (^ remove)

     */

    const getNonNullNodes = (graph) =>
      graph.nodes.filter((node) => node != null);

    graph.addEdge(root, node1);
    graph.addEdge(node1, node2);
    graph.addEdge(node2, orphan1);
    graph.addEdge(orphan1, orphan2);
    graph.addEdge(root, node3);

    assert.deepEqual(cleanUpOrphans(graph), []);
    assert.equal(getNonNullNodes(graph).length, 6);
    assert.equal(Array.from(graph.getAllEdges()).length, 5);

    graph.removeEdge(node2, orphan1, 1, false);
    assert.equal(getNonNullNodes(graph).length, 6);
    assert.equal(Array.from(graph.getAllEdges()).length, 4);

    assert.deepEqual(cleanUpOrphans(graph), [orphan1, orphan2]);
    assert.equal(getNonNullNodes(graph).length, 4);
    assert.equal(Array.from(graph.getAllEdges()).length, 3);
  });
});

describe('runInvalidation', () => {
  it('calls an invalidationFn and tracks the number of invalidated nodes', async () => {
    const mockRequestTracker = {
      getInvalidNodeCount: sinon.stub(),
    };

    mockRequestTracker.getInvalidNodeCount.returns(10000);
    const result = await runInvalidation(mockRequestTracker, {
      key: 'fsEvents',
      fn: () => {
        mockRequestTracker.getInvalidNodeCount.returns(30000);
        return {
          biggestInvalidations: [{path: 'my-file', count: 10000}],
        };
      },
    });

    assert.equal(result.key, 'fsEvents');
    assert.equal(result.count, 20000);
    assert.deepEqual(result.detail, {
      biggestInvalidations: [{path: 'my-file', count: 10000}],
    });
    assert(result.duration > 0, 'Duration was not reported');
  });
});

describe('invalidateRequestGraphFSEvents', () => {
  it('calls requestGraph.respondToFSEvents and returns the biggest invalidations', async () => {
    const requestGraph = {
      respondToFSEvents: sinon.stub(),
    };

    requestGraph.respondToFSEvents.returns({
      invalidationsByPath: new Map([
        ['file-1', 10],
        ['file-2', 5000],
        ['file-3', 8000],
      ]),
    });
    // $FlowFixMe
    const options: AtlaspackOptions = {
      unstableFileInvalidations: undefined,
    };

    const result = await invalidateRequestGraphFSEvents(requestGraph, options, [
      {
        path: 'file-1',
        type: 'create',
      },
      {
        path: 'file-2',
        type: 'update',
      },
      {
        path: 'file-3',
        type: 'delete',
      },
    ]);

    assert.deepEqual(result.biggestInvalidations, [
      {path: 'file-3', count: 8000},
      {path: 'file-2', count: 5000},
      {path: 'file-1', count: 10},
    ]);
    assert.equal(requestGraph.respondToFSEvents.callCount, 1);
    assert.deepEqual(requestGraph.respondToFSEvents.args[0], [
      [
        {path: 'file-1', type: 'create'},
        {path: 'file-2', type: 'update'},
        {path: 'file-3', type: 'delete'},
      ],
      options,
      10000,
      true,
    ]);
  });
});

describe('getBiggestFSEventsInvalidations', () => {
  it('returns the paths that invalidated the most nodes', () => {
    const invalidationsByPath = new Map([
      ['file-1', 10],
      ['file-2', 5000],
      ['file-3', 8000],
      ['file-4', 1000],
      ['file-5', 1000],
    ]);

    assert.deepEqual(getBiggestFSEventsInvalidations(invalidationsByPath, 2), [
      {path: 'file-3', count: 8000},
      {path: 'file-2', count: 5000},
    ]);
  });
});

describe('environment caching', () => {
  const env1 = {
    id: 'd821e85f6b50315e',
    context: 'browser',
    engines: {browsers: ['> 0.25%']},
    includeNodeModules: true,
    outputFormat: 'global',
    isLibrary: false,
    shouldOptimize: false,
    shouldScopeHoist: false,
    loc: undefined,
    sourceMap: undefined,
    sourceType: 'module',
    unstableSingleFileOutput: false,
  };
  const env2 = {
    id: 'de92f48baa8448d2',
    context: 'node',
    engines: {
      browsers: [],
      node: '>= 8',
    },
    includeNodeModules: false,
    outputFormat: 'commonjs',
    isLibrary: true,
    shouldOptimize: true,
    shouldScopeHoist: true,
    loc: null,
    sourceMap: null,
    sourceType: 'module',
    unstableSingleFileOutput: false,
  };

  beforeEach(async () => {
    await options.cache.ensure();

    for (const key of options.cache.keys()) {
      await options.cache.getNativeRef().delete(key);
    }
    setAllEnvironments([]);

    setFeatureFlags({
      ...DEFAULT_FEATURE_FLAGS,
      environmentDeduplication: true,
    });
  });

  it('should store environments by ID in the cache', async () => {
    await storeEnvById(options.cache, env1);

    const cachedEnv1 = await options.cache.get(
      `Environment/${ATLASPACK_VERSION}/${env1.id}`,
    );
    assert.deepEqual(cachedEnv1, env1, 'Environment 1 should be cached');
  });

  it('should list all environment IDs in the environment manager', async () => {
    const environmentIds = ['env1', 'env2'];

    await storeEnvManager(options.cache, environmentIds);

    const cachedEnvIds = await options.cache.get(
      `EnvironmentManager/${ATLASPACK_VERSION}`,
    );
    assert.deepEqual(cachedEnvIds, Array.from(environmentIds));
  });

  it('should write all environments to cache using writeEnvironments', async () => {
    setAllEnvironments([env1, env2]);
    await writeEnvironments(options.cache);

    // Verify each environment was stored individually
    const cachedEnv1 = await options.cache.get(
      `Environment/${ATLASPACK_VERSION}/${env1.id}`,
    );
    const cachedEnv2 = await options.cache.get(
      `Environment/${ATLASPACK_VERSION}/${env2.id}`,
    );
    assert.deepEqual(cachedEnv1, env1, 'Environment 1 should be cached');
    assert.deepEqual(cachedEnv2, env2, 'Environment 2 should be cached');

    // Verify environment IDs were stored in manager
    const cachedEnvIds = await options.cache.get(
      `EnvironmentManager/${ATLASPACK_VERSION}`,
    );
    const cachedIdsArray = nullthrows(cachedEnvIds);
    assert(
      cachedIdsArray.length === 2 &&
        [env1.id, env2.id].every((id) => cachedIdsArray.includes(id)),
      'Environment IDs should be stored in manager',
    );
  });

  it('should load environments from cache on loadRequestGraph on a subsequent build', async () => {
    // Simulate cache written on a first build
    await storeEnvById(options.cache, env1);
    await storeEnvById(options.cache, env2);
    await storeEnvManager(options.cache, [env1.id, env2.id]);

    await loadEnvironmentsFromCache(options.cache);

    const loadedEnvironments = getAllEnvironments();
    assert.equal(
      loadedEnvironments.length,
      2,
      'Should load 2 environments from cache',
    );

    const env1Loaded = loadedEnvironments.find((e) => e.id === env1.id);
    const env2Loaded = loadedEnvironments.find((e) => e.id === env2.id);

    assert.deepEqual(
      env1Loaded,
      env1,
      'First environment should match cached environment',
    );
    assert.deepEqual(
      env2Loaded,
      env2,
      'Second environment should match cached environment',
    );
  });

  it('should handle empty cache gracefully without calling setAllEnvironments', async () => {
    const setAllEnvironmentsSpy = sinon.spy(setAllEnvironments);

    await assert.doesNotReject(
      loadEnvironmentsFromCache(options.cache),
      'loadEnvironmentsFromCache should not throw when cache is empty',
    );

    assert.equal(
      setAllEnvironmentsSpy.callCount,
      0,
      'setAllEnvironments should not be called when loading from empty cache',
    );
  });

  it('should not load environments from a different version', async () => {
    const setAllEnvironmentsSpy = sinon.spy(setAllEnvironments);
    const differentVersion = '2.17.2'; // A different version than ATLASPACK_VERSION

    // Store an environment with a different version
    await options.cache.set(`Environment/${differentVersion}/${env1.id}`, env1);
    await options.cache.set(`EnvironmentManager/${differentVersion}`, [
      env1.id,
    ]);

    await loadEnvironmentsFromCache(options.cache);

    assert.equal(
      setAllEnvironmentsSpy.callCount,
      0,
      'setAllEnvironments should not be called when loading from different version',
    );
    const loadedEnvironments = getAllEnvironments();
    assert.equal(
      loadedEnvironments.length,
      0,
      'Should not load any environments from different version',
    );
  });
});
