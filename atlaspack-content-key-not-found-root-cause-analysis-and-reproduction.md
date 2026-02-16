# Root Cause Analysis: "Graph already has content key" Error

## Summary

The "Graph already has content key" error is caused by a **shared mutable state bug** in `AssetGraphRequestRust.ts`. When `getAssetGraph()` builds an incremental asset graph from a previous graph, it passes the previous graph's `_contentKeyToNodeId` Map, `_nodeIdToContentKey` Map, and `nodes` array **by reference** into the new graph. Any mutations to the new graph (adding new nodes, new content keys) silently corrupt the previous graph. If a build subsequently fails downstream (e.g., during symbol propagation), the corrupted previous graph is reused for the next rebuild, which then throws "Graph already has content key" when it encounters content keys that were never supposed to be there.

This error was introduced with Atlaspack v3's `AssetGraphRequestRust` code path and is not present in v2, because v2's `AssetGraph.addNode()` gracefully handles duplicate content keys.

## Error Details

- **Error message**: `Error: Graph already has content key <hash>`
- **Origin**: `ContentGraph.addNodeByContentKey()` in `@atlaspack/graph`
- **Affected path**: Atlaspack v3 watch mode (Confluence dev server)
- **Impact**: Over 4,000 Sentry events, 169 affected users
- **Sentry**: [Issue list](https://atlassian-2y.sentry.io/explore/discover/results/?dataset=errors&display=default&field=issue&field=title&field=count%28%29&field=count_unique%28user%29&field=project&name=Frequent%20Unhandled%20Issues&project=5988835&query=event.type%3Aerror%20graph%20already%20has%20content%20key&queryDataset=error-events&sort=-count&statsPeriod=90d&yAxis=count%28%29)

## The Bug

### Where it happens

In `packages/core/core/src/requests/AssetGraphRequestRust.ts`, the `getAssetGraph()` function has three code paths for constructing an `AssetGraph`:

1. **Path 1 (safeToSkipBundling)**: Reuses everything from prevAssetGraph including edges
2. **Path 2 (incremental rebuild)**: Reuses nodes and content key Maps from prevAssetGraph, rebuilds edges
3. **Path 3 (fresh build)**: Creates everything from scratch with new Maps

Both Path 1 and Path 2 share the same critical defect:

```typescript
// AssetGraphRequestRust.ts, lines 152-160 (Path 2 shown)
graph = new AssetGraph({
  _contentKeyToNodeId: prevAssetGraph._contentKeyToNodeId, // <-- SHARED reference
  _nodeIdToContentKey: prevAssetGraph._nodeIdToContentKey, // <-- SHARED reference
  nodes: prevAssetGraph.nodes, // <-- SHARED reference
  initialCapacity: serializedGraph.edges.length,
  initialNodeCapacity: prevAssetGraph.nodes.length + 1,
  rootNodeId: prevAssetGraph.rootNodeId,
});
```

The new `AssetGraph` instance and the `prevAssetGraph` point to the **same JavaScript objects** for `_contentKeyToNodeId`, `_nodeIdToContentKey`, and `nodes`. This is not a copy -- it is an alias.

### Why the shared references are destructive

When the new graph processes serialized nodes from Rust, new nodes are added via `updateNode()`:

```typescript
// AssetGraphRequestRust.ts, lines 233-243
function updateNode(newNode: AssetGraphNode, isUpdateNode: boolean) {
  if (isUpdateNode) {
    let existingNode = graph.getNodeByContentKey(newNode.id);
    assert(existingNode && existingNode.type === newNode.type);
    Object.assign(existingNode, newNode);
  } else {
    graph.addNodeByContentKey(newNode.id, newNode); // <-- writes to the SHARED Map
  }
}
```

For new nodes (`isUpdateNode=false`), `addNodeByContentKey` is called, which:

1. Adds the content key to `this._contentKeyToNodeId` -- but this is the **same Map** as `prevAssetGraph._contentKeyToNodeId`
2. Pushes the node to `this.nodes` -- but this is the **same Array** as `prevAssetGraph.nodes`

After `getAssetGraph()` completes, `prevAssetGraph` now contains content keys and nodes that it never had originally. It has been silently corrupted.

### The chain of references

To understand why the Maps are shared, trace the constructor chain:

```
getAssetGraph() passes { _contentKeyToNodeId: prevAssetGraph._contentKeyToNodeId, ... }
  → new AssetGraph(opts)
    → ContentGraph constructor: this._contentKeyToNodeId = opts._contentKeyToNodeId  // direct assignment
    → Graph constructor: this.nodes = opts.nodes ?? []                               // direct assignment
```

At no point is a copy made. `ContentGraph` (in `packages/core/graph/src/ContentGraph.ts`) assigns `this._contentKeyToNodeId = _contentKeyToNodeId` directly. `Graph` (in `packages/core/graph/src/Graph.ts`) assigns `this.nodes = nodes ?? []` directly.

### How the error manifests in production

The error requires a two-step failure cascade:

```
Build N: Initial build succeeds.
         prevAssetGraph is stored in RequestTracker.

Build N+1 (watch rebuild):
  1. Rust builds the asset graph, sends serialized delta to JS.
  2. getAssetGraph(serialized, prevAssetGraph) runs.
     - Creates new graph sharing prevAssetGraph's Maps and nodes.
     - New nodes are added → prevAssetGraph's Maps are silently mutated.
  3. Something AFTER getAssetGraph fails (e.g., symbol propagation error,
     assertion failure, OOM, etc.).
  4. Because the build failed, storeResult is never called.
     prevAssetGraph (now corrupted) remains in RequestTracker.

Build N+2 (watch rebuild):
  1. Rust sends the same (or similar) serialized delta.
  2. getAssetGraph(serialized, prevAssetGraph) runs again.
     - prevAssetGraph._contentKeyToNodeId already contains the "new" content
       keys from Build N+1's failed attempt.
     - addNodeByContentKey() finds the key already exists.
     - THROWS: "Graph already has content key <hash>"
```

The error is self-reinforcing: once triggered, every subsequent rebuild will fail with the same error because the corrupted prevAssetGraph is never replaced. The only recovery is to restart the dev server.

## Why v2 doesn't have this bug

In v2, the `AssetGraph.addNode()` method (in `packages/core/core/src/AssetGraph.ts`) has a defensive check:

```typescript
// AssetGraph.ts, lines 269-280
addNode(node: AssetGraphNode): NodeId {
  this.hash = null;
  let existing = this.getNodeByContentKey(node.id);
  if (existing != null) {
    // Gracefully update the existing node instead of throwing
    invariant(existing.type === node.type);
    existing.value = node.value;
    let existingId = this.getNodeIdByContentKey(node.id);
    this.updateNode(existingId, existing);
    return existingId;
  }
  return super.addNodeByContentKey(node.id, node);
}
```

v2 checks for duplicates first and updates in place. v3's `AssetGraphRequestRust` bypasses this by calling `ContentGraph.addNodeByContentKey()` directly, which throws unconditionally on duplicates.

## Reproduction

### Unit test reproduction

Two unit tests were added to `packages/core/core/test/requests/AssetGraphRequestRust.test.ts` that reproduce the bug directly against the `getAssetGraph()` function.

#### Test 1: "should not mutate prevAssetGraph when building a new graph"

This test proves the root cause -- that calling `getAssetGraph` with a `prevAssetGraph` mutates the previous graph's internal data structures.

```typescript
it('should not mutate prevAssetGraph when building a new graph', () => {
  // Build initial graph (fresh, no prev)
  const {assetGraph: prevGraph} = getAssetGraph(getSerializedGraph());
  const originalContentKeyCount = prevGraph._contentKeyToNodeId.size;
  const originalNodesLength = prevGraph.nodes.length;

  // Build incremental graph that adds new nodes
  const serialized = getSerializedGraphWithNewNode();
  getAssetGraph(serialized, prevGraph);

  // Verify prevGraph was NOT mutated
  assert.equal(prevGraph._contentKeyToNodeId.size, originalContentKeyCount);
  assert.equal(prevGraph.nodes.length, originalNodesLength);
});
```

**Result**: FAILS.

```
Expected value to be equal to: 7
Received: 9
Message: prevAssetGraph._contentKeyToNodeId was mutated: had 7 keys, now has 9
```

The Map started with 7 content keys (@@root + 6 nodes from the base graph). After calling `getAssetGraph(serialized, prevGraph)`, it has 9 keys -- the 2 extra keys (`dd00000000000001` and `dd00000000000002`) are the new dependency and asset nodes that were added by the incremental build. These were written directly into `prevGraph._contentKeyToNodeId` because the new graph shares the same Map object.

#### Test 2: "should not throw 'Graph already has content key' on retry after simulated failure"

This test reproduces the exact production error. It simulates the two-step failure cascade: a build that succeeds at the `getAssetGraph` level but "fails" downstream (simulated by simply not calling `storeResult`), followed by a retry that hits the corrupted state.

```typescript
it('should not throw "Graph already has content key" on retry after simulated failure', () => {
  // Build initial graph (fresh, no prev)
  const {assetGraph: prevGraph} = getAssetGraph(getSerializedGraph());

  // Simulate build N+1: getAssetGraph succeeds, but something after it fails.
  // getAssetGraph mutates prevGraph's Maps because they are shared by reference.
  const serialized = getSerializedGraphWithNewNode();
  getAssetGraph(serialized, prevGraph); // succeeds, but mutates prevGraph

  // Simulate build N+2: same serialized graph, prevGraph reused after failure.
  assert.doesNotThrow(() => {
    getAssetGraph(serialized, prevGraph);
  }, /Graph already has content key/);
});
```

**Result**: FAILS with the exact production error.

```
Expected the function not to throw an error.
Instead, it threw: [Error: Graph already has content key dd00000000000001]
```

The first call to `getAssetGraph(serialized, prevGraph)` added content key `dd00000000000001` to `prevGraph._contentKeyToNodeId`. The second call finds that key already present and `ContentGraph.addNodeByContentKey()` throws. This is exactly what happens in production when a watch rebuild fails and the next rebuild reuses the corrupted graph.

### Helper function: `getSerializedGraphWithNewNode()`

The tests use a helper that returns a serialized graph representing an incremental build. It adds:

- **`nodes`** (new nodes from Rust): a dependency for `./utils` and an asset for `/utils.ts`
- **`updates`** (changed existing nodes): the existing `/index.ts` asset, marked as rebuilt
- **`edges`**: the complete edge list including connections to the new nodes

This simulates what Rust sends to JS when a file change adds a new import.

## End-to-End Trigger Analysis

### Why the exact error is hard to trigger in integration tests

Deep analysis of the Rust-JS interaction reveals that in the **normal** propagateSymbols failure path, the exact "Graph already has content key" error does not manifest end-to-end. This is because:

1. **Rust always caches its graph after a successful build.** When `build_asset_graph()` succeeds (which happens before JS processing), the result is stored in Rust's request tracker via `store_request(Valid(result))` ([request_tracker.rs:286](crates/atlaspack/src/request_tracker/request_tracker.rs)).

2. **Previously-new nodes are never sent as "new" again.** On the next build, `get_cached_request_result()` returns the latest cached graph ([line 330](crates/atlaspack/src/request_tracker/request_tracker.rs)). `AssetGraph::from(prev)` clones this and sets `starting_node_count` to the previous node count. Nodes below `starting_node_count` appear in `updated_nodes()` (if changed), not `new_nodes()`.

3. **Updates use `Object.assign`, not `addNodeByContentKey`.** When a node comes through as an update (`isUpdateNode=true`), the `updateNode` function calls `getNodeByContentKey` + `Object.assign`, which never throws on duplicate keys.

The result: even though Build N+1's `getAssetGraph` corrupts `prevAssetGraph`'s Maps (adding ghost content keys), Build N+2 gets these same nodes as `updates` from Rust (because Rust committed them). The `Object.assign` path handles them without error.

### When the error CAN manifest

The error requires **Rust and JS to disagree** about which content keys exist. Specifically, Rust must send a content key as a "new" node (in `serializedGraph.nodes`) while JS already has that key in its corrupted `_contentKeyToNodeId` Map. This state desync can occur during:

- **Build aborts at specific timing**: If a `BuildAbortError` is thrown after `getAssetGraph` has corrupted the Maps but before `storeResult` is called, AND the Rust commit thread hasn't finished yet, the next build could see Rust start from an older state while JS has the corrupted Maps.
- **Race conditions between concurrent builds**: Rapid file changes could cause overlapping builds where the shared Maps are mutated by multiple concurrent `getAssetGraph` calls.
- **Process crashes**: If the Node.js process crashes after `getAssetGraph` mutates the Maps but before the Rust commit thread completes, subsequent builds after restart could hit the desync.

### Integration test coverage

Three v3 integration tests were added to exercise the corruption path:

1. **"should recover after a symbol propagation error without graph corruption"** (existing fixture: `update-used-symbols-remove-export`): Tests recovery after a build failure caused by a removed export. This test PASSES because no new nodes are added (only existing nodes are updated).

2. **"should recover after adding new files that cause a symbol error"** (new fixture: `v3-graph-corruption-recovery`): Tests recovery after a build failure that involves **new files** (`c.js`, `d.js`) being added to the graph. This exercises the exact code path where new content keys are written to the shared Maps via `addNodeByContentKey`. While the test passes today (because Rust sends these nodes as updates on the recovery build), it serves as a regression test: after the fix is applied (cloning Maps), this test validates that new-node + failure + recovery works correctly end-to-end.

3. **"should recover from rapid file changes that add and remove new imports"** (same fixture): An opportunistic test that makes rapid file changes without awaiting between them, attempting to trigger a build abort at the right time. This exercises the build-abort code path where the exact error is most likely to manifest.

The **unit tests remain the definitive proof** of the shared mutable state bug. The integration tests provide additional coverage for the recovery path and serve as regression tests for the fix.

## Proposed Fixes

The fix should ensure that `getAssetGraph()` does not share mutable state with `prevAssetGraph`. The most direct approach:

### Option A: Clone the shared structures (recommended)

Replace the direct reference assignments with shallow copies:

```typescript
graph = new AssetGraph({
  _contentKeyToNodeId: new Map(prevAssetGraph._contentKeyToNodeId), // clone
  _nodeIdToContentKey: new Map(prevAssetGraph._nodeIdToContentKey), // clone
  nodes: [...prevAssetGraph.nodes], // clone
  initialCapacity: serializedGraph.edges.length,
  initialNodeCapacity: prevAssetGraph.nodes.length + 1,
  rootNodeId: prevAssetGraph.rootNodeId,
});
```

This applies to both Path 1 (safeToSkipBundling) and Path 2 (incremental rebuild).

**Pros**: Simple, isolated change. No downstream modifications needed. Both unit tests pass with this fix.

**Cons**: Small memory/time overhead for cloning. For large graphs (Confluence has thousands of nodes), cloning a Map and Array on every rebuild adds some cost, but this is negligible compared to the full build.

### Option B: Make `updateNode` defensive

Add a duplicate check in `updateNode` before calling `addNodeByContentKey`:

```typescript
function updateNode(newNode: AssetGraphNode, isUpdateNode: boolean) {
  if (isUpdateNode || graph.hasContentKey(newNode.id)) {
    let existingNode = graph.getNodeByContentKey(newNode.id);
    assert(existingNode && existingNode.type === newNode.type);
    Object.assign(existingNode, newNode);
  } else {
    graph.addNodeByContentKey(newNode.id, newNode);
  }
}
```

**Pros**: Prevents the throw. Matches v2's behavior.

**Cons**: Masks the underlying data corruption. The prevAssetGraph is still being mutated; the error is just suppressed. This could lead to subtle correctness issues.

### Recommendation

**Option A is the correct fix.** It addresses the root cause (shared mutable state) rather than masking the symptom. Option B could be applied as additional defense-in-depth, but should not be the sole fix.

Both fixes should be applied to both Path 1 and Path 2 in `getAssetGraph()`.
