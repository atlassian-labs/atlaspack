// @flow strict-local

import assert from 'assert';
import nullthrows from 'nullthrows';
import RequestTracker, {
  type RunAPI,
  cleanUpOrphans,
  runInvalidation,
  getBiggestFSEventsInvalidations,
  invalidateRequestGraphFSEvents,
  requestTypes,
} from '../src/RequestTracker';
import {Graph} from '@atlaspack/graph';
import {LMDBLiteCache} from '@atlaspack/cache';
import WorkerFarm from '@atlaspack/workers';
import {DEFAULT_OPTIONS} from './test-utils';
import {FILE_CREATE, FILE_UPDATE, INITIAL_BUILD} from '../src/constants';
import {makeDeferredWithPromise} from '@atlaspack/utils';
import {toProjectPath} from '../src/projectPath';
import {DEFAULT_FEATURE_FLAGS, setFeatureFlags} from '../../feature-flags/src';
import sinon from 'sinon';
import type {AtlaspackOptions} from '../src/types';
import createAtlaspackBuildRequest from '../src/requests/AtlaspackBuildRequest';
import Atlaspack from '../src/Atlaspack';
import {MemoryFS, NodeFS} from '@atlaspack/fs';
import {OverlayFS} from '../../fs/src/OverlayFS';
import path from 'path';
import createAssetGraphRequest from '../src/requests/AssetGraphRequest';

const options = {
  ...DEFAULT_OPTIONS,
  cache: new LMDBLiteCache(DEFAULT_OPTIONS.cacheDir),
};

describe('RequestTracker', () => {
  let farm;
  before(() => {
    farm = new WorkerFarm({workerPath: require.resolve('../src/worker')});
  });

  beforeEach(async () => {
    await options.cache.ensure();

    for (const key of options.cache.keys()) {
      await options.cache.getNativeRef().delete(key);
    }
  });

  after(() => {
    farm.end();
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

  describe('incremental bundling', () => {
    async function runIncrementalBundlingScenario(
      incrementalBundlingVersioning: boolean,
    ) {
      const fs = new OverlayFS(new MemoryFS(farm), new NodeFS());
      const appRoot = __dirname;
      await fs.mkdirp(path.join(appRoot, 'app'));
      await fs.writeFile(path.join(appRoot, 'app', 'package.json'), '{}');
      await fs.writeFile(path.join(appRoot, 'app', '.git'), '');
      await fs.writeFile(
        path.join(appRoot, 'app', '.parcelrc'),
        '{"extends":"@atlaspack/config-default"}',
      );
      await fs.writeFile(
        path.join(appRoot, 'app', 'target.js'),
        'console.log("hello")',
      );

      const atlaspack = new Atlaspack({
        featureFlags: {
          incrementalBundlingVersioning,
        },
        workerFarm: farm,
        entries: [path.join(appRoot, 'app', 'target.js')],
        cache: new LMDBLiteCache(DEFAULT_OPTIONS.cacheDir),
        inputFS: fs,
        outputFS: fs,
      });
      await atlaspack._init();
      const options = atlaspack._getResolvedAtlaspackOptions();
      const tracker = new RequestTracker({farm, options});
      let {ref: optionsRef} = await farm.createSharedReference(options, false);

      const getAssetRequests = () =>
        runRequestSpy
          .getCalls()
          .map((call) => call.args[0])
          .filter((request) => request.type === requestTypes.asset_request);

      // Running the build once builds one asset
      const runRequestSpy = sinon.spy(tracker, 'runRequest');
      await tracker.runRequest(
        createAtlaspackBuildRequest({
          optionsRef,
          requestedAssetIds: new Set(),
        }),
      );
      assert.equal(getAssetRequests().length, 1);
      runRequestSpy.resetHistory();

      // Running the build again with no invalidations does not build any assets
      await tracker.runRequest(
        createAtlaspackBuildRequest({
          optionsRef,
          requestedAssetIds: new Set(),
        }),
      );
      assert.equal(getAssetRequests().length, 0);
      runRequestSpy.resetHistory();

      // Running the build again with a file change builds the asset again
      tracker.respondToFSEvents(
        [
          {
            type: 'update',
            path: path.join(appRoot, 'app', 'target.js'),
          },
        ],
        Number.MAX_VALUE,
      );
      await tracker.runRequest(
        createAtlaspackBuildRequest({
          optionsRef,
          requestedAssetIds: new Set(),
        }),
      );
      assert.equal(getAssetRequests().length, 1);
      runRequestSpy.resetHistory();

      // Run the asset graph request, but not bundling
      await fs.writeFile(
        path.join(appRoot, 'app', 'target.js'),
        'require("./dep.js")',
      );
      await fs.writeFile(
        path.join(appRoot, 'app', 'dep.js'),
        'console.log("dep")',
      );
      tracker.respondToFSEvents(
        [
          {
            type: 'update',
            path: path.join(appRoot, 'app', 'target.js'),
          },
        ],
        Number.MAX_VALUE,
      );
      const assetGraphRequestResult = await tracker.runRequest(
        createAssetGraphRequest({
          name: 'Main',
          entries: [
            toProjectPath(
              path.join(appRoot, 'app'),
              path.join(appRoot, 'app', 'target.js'),
            ),
          ],
          optionsRef,
          shouldBuildLazily: false,
          lazyIncludes: [],
          lazyExcludes: [],
          requestedAssetIds: new Set(),
        }),
      );
      assert.equal(getAssetRequests().length, 2);
      assert.equal(
        assetGraphRequestResult.assetGraph.safeToIncrementallyBundle,
        false,
      );

      // Now make another change
      tracker.respondToFSEvents(
        [
          {
            type: 'update',
            path: path.join(appRoot, 'app', 'target.js'),
          },
          {
            type: 'update',
            path: path.join(appRoot, 'app', 'dep.js'),
          },
        ],
        Number.MAX_VALUE,
      );
      // And run the build again

      if (!incrementalBundlingVersioning) {
        await assert.rejects(async () => {
          await tracker.runRequest(
            createAtlaspackBuildRequest({
              optionsRef,
              requestedAssetIds: new Set(),
            }),
          );
        });
      } else {
        await tracker.runRequest(
          createAtlaspackBuildRequest({
            optionsRef,
            requestedAssetIds: new Set(),
          }),
        );
      }
    }

    it('throws a content key not found exception without bundling versioning', async () => {
      await runIncrementalBundlingScenario(false);
    });

    it('works fine with bundling versioning', async () => {
      await runIncrementalBundlingScenario(true);
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
