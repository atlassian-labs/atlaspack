import assert from 'assert';
import nullthrows from 'nullthrows';
import RequestTracker, {RunAPI, cleanUpOrphans} from '../src/RequestTracker';
import {Graph} from '@atlaspack/graph';
import WorkerFarm from '@atlaspack/workers';
import {DEFAULT_OPTIONS} from './test-utils';
import {INITIAL_BUILD} from '../src/constants';
import {makeDeferredWithPromise} from '@atlaspack/utils';

const options = DEFAULT_OPTIONS;
// @ts-expect-error - TS2345 - Argument of type '{ workerPath: string; }' is not assignable to parameter of type 'FarmOptions'.
const farm = new WorkerFarm({workerPath: require.resolve('../src/worker.js')});

describe('RequestTracker', () => {
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
    // @ts-expect-error - TS2367 - This condition will always return 'false' since the types 'false' and 'true' have no overlap.
    assert(called === true);
  });

  it('should invalidate requests with invalidated subrequests', async () => {
    let tracker = new RequestTracker({farm, options});
    await tracker.runRequest({
      id: 'abc',
      type: 7,
      run: async ({api}: any) => {
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
        .map((req: any) => req.id)
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
        .map((req: any) => req.id)
        .includes('abc'),
    );
  });

  it('should remove subrequests that are no longer called within a request', async () => {
    let tracker = new RequestTracker({farm, options});
    await tracker.runRequest({
      id: 'abc',
      type: 7,
      run: async ({api}: any) => {
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
      run: async ({api}: any) => {
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
      // @ts-expect-error - TS2322 - Type '({ api }: { api: RunAPI<string | undefined>; }) => Promise<void>' is not assignable to type '(arg1: { input: null; } & StaticRunOpts<string | undefined>) => Async<string | undefined>'.
      run: async ({api}: {api: RunAPI<string | undefined>}) => {
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
    // @ts-expect-error - TS2367 - This condition will always return 'false' since the types 'void' and 'string' have no overlap.
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
    // @ts-expect-error - TS2345 - Argument of type '{ aborted: true; }' is not assignable to parameter of type 'AbortSignal'.
    tracker.setSignal({aborted: true});
    await p;
    assert(
      tracker
        .getInvalidRequests()
        .map((req: any) => req.id)
        .includes('abc'),
    );
  });

  it('should write cache to disk and store index', async () => {
    let tracker = new RequestTracker({farm, options});

    await tracker.runRequest({
      id: 'abc',
      type: 7,
      // $FlowFixMe string isn't a valid result
      // @ts-expect-error - TS2322 - Type '({ api }: { api: RunAPI<string | undefined>; }) => Promise<void>' is not assignable to type '(arg1: { input: null; } & StaticRunOpts<string | undefined>) => Async<string | undefined>'.
      run: async ({api}: {api: RunAPI<string | undefined>}) => {
        let result = await Promise.resolve();
        // @ts-expect-error - TS2345 - Argument of type 'void' is not assignable to parameter of type 'string | undefined'.
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
      run: async ({api}: {api: RunAPI<string>}) => {
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
      run: async ({api}: {api: RunAPI<string>}) => {
        calledB = true;
        await lockB.promise;
        api.storeResult('b');
        return 'b';
      },
      input: null,
    });

    // @ts-expect-error - TS2554 - Expected 1 arguments, but got 0.
    lockA.deferred.resolve();
    // @ts-expect-error - TS2554 - Expected 1 arguments, but got 0.
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
      // @ts-expect-error - TS2322 - Type '({ api }: { api: RunAPI<string | undefined>; }) => Promise<void>' is not assignable to type '(arg1: { input: null; } & StaticRunOpts<string | undefined>) => Async<string | undefined>'.
      run: async ({api}: {api: RunAPI<string | undefined>}) => {
        await lockB.promise;
        api.storeResult('b');
      },
      input: null,
    });

    // @ts-expect-error - TS2554 - Expected 1 arguments, but got 0.
    lockA.deferred.resolve();
    // @ts-expect-error - TS2554 - Expected 1 arguments, but got 0.
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
      // @ts-expect-error - TS2322 - Type '({ api }: { api: RunAPI<string | undefined>; }) => Promise<void>' is not assignable to type '(arg1: { input: null; } & StaticRunOpts<string | undefined>) => Async<string | undefined>'.
      run: async ({api}: {api: RunAPI<string | undefined>}) => {
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
        // @ts-expect-error - TS2322 - Type '({ api }: { api: RunAPI<string | undefined>; }) => Promise<void>' is not assignable to type '(arg1: { input: null; } & StaticRunOpts<string | undefined>) => Async<string | undefined>'.
        run: async ({api}: {api: RunAPI<string | undefined>}) => {
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
    // @ts-expect-error - TS2339 - Property 'invalidateReason' does not exist on type 'RequestGraphNode'.
    assert.equal(tracker.graph.getNode(nodeId)?.invalidateReason, 0);
    await tracker.writeToCache();

    tracker.graph.invalidateNode(nodeId, 1);
    // @ts-expect-error - TS2339 - Property 'invalidateReason' does not exist on type 'RequestGraphNode'.
    assert.equal(tracker.graph.getNode(nodeId)?.invalidateReason, 1);
    await tracker.writeToCache();

    // Create a new tracker from cache
    tracker = await RequestTracker.init({farm, options});

    // @ts-expect-error - TS2339 - Property 'invalidateReason' does not exist on type 'RequestGraphNode'.
    assert.equal(tracker.graph.getNode(nodeId)?.invalidateReason, 1);
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

    const getNonNullNodes = (graph: any) =>
      graph.nodes.filter((node: any) => node != null);

    graph.addEdge(root, node1);
    graph.addEdge(node1, node2);
    graph.addEdge(node2, orphan1);
    graph.addEdge(orphan1, orphan2);
    graph.addEdge(root, node3);

    assert.deepEqual(cleanUpOrphans(graph), []);
    assert.equal(getNonNullNodes(graph).length, 6);
    // @ts-expect-error - TS2769 - No overload matches this call.
    assert.equal(Array.from(graph.getAllEdges()).length, 5);

    graph.removeEdge(node2, orphan1, 1, false);
    assert.equal(getNonNullNodes(graph).length, 6);
    // @ts-expect-error - TS2769 - No overload matches this call.
    assert.equal(Array.from(graph.getAllEdges()).length, 4);

    assert.deepEqual(cleanUpOrphans(graph), [orphan1, orphan2]);
    assert.equal(getNonNullNodes(graph).length, 4);
    // @ts-expect-error - TS2769 - No overload matches this call.
    assert.equal(Array.from(graph.getAllEdges()).length, 3);
  });
});
