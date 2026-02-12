# Native Bundler Rust Rewrite - Working Session Tracker

## Overview

This document tracks ongoing work on the Rust bundler rewrite for Atlaspack. It serves as a persistent context document that can be used to recover state if the LLM context window is lost.

**Current focus**: Getting native bundler parity integration tests passing.

---

## Task 1: Parity Integration Tests

**Test file**: `packages/core/integration-tests/test/native-bundler-parity.ts`
**How to run**: `ATLASPACK_V3=true yarn workspace @atlaspack/integration-tests test -- --grep "Native bundling ready" --timeout 120000`
**Status**: 9 passing, 2 failing (as of 2026-02-12, after Phase 7 + type-change fix)

## Architecture Summary

The pipeline for native bundling is:

1. **Rust IdealGraphBundler** (`crates/atlaspack_bundling/src/ideal_graph/`) builds an `IdealGraph` from the `AssetGraph`
2. **`Bundler::bundle()`** in `ideal_graph/mod.rs` materializes the `IdealGraph` into a `NativeBundleGraph` (creates bundle nodes, bundle groups, edges)
3. **Serialization** in `crates/node-bindings/src/atlaspack/serialize_bundle_graph.rs` converts `NativeBundleGraph` to JSON buffers for JS
4. **`BundleGraphRequestRust.ts`** (`packages/core/core/src/requests/BundleGraphRequestRust.ts`) deserializes and reconstructs the JS-side `InternalBundleGraph`
5. **Naming + Runtimes** are applied on the JS side (namers, `applyRuntimes`)

The runtimes step adds assets like `esmodule-helpers.js`, `bundle-url.js`, `cacheLoader.js`, `js-loader.js`, `bundle-manifest.js` to bundles that need them.

## Key Files

| File                                                            | Purpose                                                       |
| --------------------------------------------------------------- | ------------------------------------------------------------- |
| `crates/atlaspack_bundling/src/ideal_graph/mod.rs`              | IdealGraph algorithm + materialization into NativeBundleGraph |
| `crates/atlaspack_bundling/src/ideal_graph/builder.rs`          | IdealGraphBuilder - builds the ideal graph from asset graph   |
| `crates/atlaspack_bundling/src/ideal_graph/types.rs`            | IdealGraph, IdealBundle, IdealBundleId types                  |
| `crates/node-bindings/src/atlaspack/serialize_bundle_graph.rs`  | Serialization of NativeBundleGraph to JS                      |
| `packages/core/core/src/requests/BundleGraphRequestRust.ts`     | JS-side deserialization + naming + runtimes                   |
| `packages/core/core/src/BundleGraph.ts`                         | InternalBundleGraph - `isAssetReachableFromBundle` method     |
| `packages/core/core/src/applyRuntimes.ts`                       | Runtime asset injection (esmodule-helpers, bundle-url, etc.)  |
| `packages/core/integration-tests/test/native-bundler-parity.ts` | The parity test suite                                         |

## Feature Flag

Tests use `setupV3Flags({nativeBundling: true})` which sets `ATLASPACK_V3=true` and enables the `nativeBundling` feature flag. The `describe.v3` / `it.v3` helpers in test-utils only run when `ATLASPACK_V3=true`.

---

## Passing Tests (7)

1. `should not split any bundles when using singleFileOutput` (MonolithicBundler path)
2. `bundles and runs a simple entry` - single entry, no async
3. `supports multiple entries` - two entries, no shared deps
4. `creates a shared bundle for a common dependency` - two async imports share one dep
5. `creates a shared bundle for multiple common dependencies` - two async imports share two deps
6. `duplicates shared sync dep into all entry bundles` - multi-entry with shared sync dep
7. `suppresses shared extraction when asset is available from ancestor` - availability propagation works

---

## Failing Tests (4)

### FAIL 1: `supports dynamic import`

**Expected**:

```
entry bundle: [index.js, esmodule-helpers.js, bundle-url.js, cacheLoader.js, js-loader.js, bundle-manifest.js]
async bundle: [async.js]
```

**Actual**:

```
entry bundle: [index.js, esmodule-helpers.js, bundle-url.js, cacheLoader.js, js-loader.js, bundle-manifest.js]  OK
async bundle: [async.js, esmodule-helpers.js]  WRONG (extra esmodule-helpers.js)
```

**Investigation history** (2026-02-12):

1. Initial hypothesis: Rust adds `root -> async_bundle_group Bundle(3)` edges - DISPROVEN by serialization debug.
2. Rust serialized Bundle(3) edges are correct: `0->8 (root->entry_bg)`, `8->9 (entry_bg->entry_bundle)`, `9->11 (entry_bundle->async_bg)`, `11->10 (async_bg->async_bundle)`.
3. A phantom `root(0) -> async_bundle_group(11) Bundle(3)` edge appears AFTER `applyRuntimes` runs JS-side.
4. Added root-node filtering in `isAssetReachableFromBundle` - did NOT fix test failures. Non-v3 tests still pass (73/73).
5. This means either (a) `isAssetReachableFromBundle` is not the code path that decides to add `esmodule-helpers.js`, or (b) `esmodule-helpers.js` is placed into the async bundle by the Rust ideal graph builder's Contains edges BEFORE runtimes run.

**Changes applied so far** (in `crates/atlaspack_bundling/src/ideal_graph/mod.rs`):

- Async bundle groups now parented by their parent bundle via Bundle edges (not root)
- `is_splittable` now derived from asset graph (was hardcoded to false)

**Changes applied so far** (in `packages/core/core/src/BundleGraph.ts`):

- Added root-node filtering in `isAssetReachableFromBundle` parentBundleNodes

**CONFIRMED**: `esmodule-helpers.js` is in the async bundle BEFORE `applyRuntimes` runs. Debug output:

- BEFORE runtimes: entry bundle has `[index.js, esmodule-helpers.js]`, async bundle has `[async.js, esmodule-helpers.js]`
- AFTER runtimes: entry bundle gets runtime assets added, async bundle unchanged

This means the Rust materialization's Contains traversal is placing `esmodule-helpers.js` into the async bundle. The fix must be in `crates/atlaspack_bundling/src/ideal_graph/mod.rs`, in the code that adds Contains edges for bundles (around lines 420-468, the `add_asset_to_bundle` / Contains traversal section).

**Status**: FIXED - Phase 7 `place_single_root_assets` now skips splittable-root placement when asset is reachable from an actual entry root. Test now passes.

---

### FAIL 2: `internalizes async bundle when root is already sync-available`

**Expected**:

```
entry bundle: [index.js, a.js, b.js, esmodule-helpers.js]   (single bundle, b.js internalized)
```

**Actual**:

```
entry bundle: [index.js, a.js, esmodule-helpers.js, bundle-url.js, cacheLoader.js, js-loader.js, bundle-manifest.js]
separate bundle: [b.js, esmodule-helpers.js]
```

**Analysis**: `b.js` is both sync-imported (via `a.js -> b.js`) and lazy-imported (via `a.js -> import('./b')`). Since it's synchronously available from the entry, the lazy import should be "internalized" - the async bundle for b.js should be removed and the dynamic import should resolve to the already-loaded module. Instead, a separate bundle is being created for b.js.

**Root cause hypothesis**: The Rust ideal graph builder may not be performing internalization of async bundles. The V2 bundler has explicit internalization logic that detects when an async bundle's root asset is already synchronously available from every bundle that references it, and removes the async bundle. This logic may be missing or incomplete in the Rust implementation.

**Status**: NOT STARTED

---

### FAIL 3: `creates separate bundle for CSS type-change dependency`

**Expected**:

```
entry JS bundle: [index.js, esmodule-helpers.js]
CSS bundle: [styles.css]
```

**Actual**:

```
entry JS bundle: [index.js, esmodule-helpers.js, bundle-url.js]  WRONG (extra bundle-url.js)
CSS bundle: [styles.css]  OK
```

**Analysis**: The entry bundle has an extra `bundle-url.js`. This asset is injected by the JS runtime when a bundle has child bundles that need to be loaded at runtime. For a CSS sibling bundle (loaded via link tag, not JS loader), `bundle-url.js` should not be needed. This suggests the bundle graph edges are making the runtimes think the CSS bundle needs JS-based loading.

**Investigation findings**:

- Edge from JS bundle to CSS bundle group is now `References(4)` (not `Bundle(3)`) after our fix
- BUT `traverseBundles`/`getChildBundles` traverse BOTH `bundle` AND `references` edges
- So CSS bundle still appears as "child" of JS entry bundle, triggering `bundle-url.js`/`bundle-manifest.js`
- V2 bundler passes this test (in 73 passing non-v3 tests), so V2 must handle the graph differently
- Key question: Does V2 even create a `JS_bundle -> CSS_bundle_group` edge? Or is the CSS bundle a sibling within the same bundle group?

**ROOT CAUSE CONFIRMED**: V2 does NOT create a bundle group for CSS type-change bundles. V2 creates a `JS_bundle -> CSS_bundle references(4)` edge (bundle-to-bundle, no bundle group). The CSS bundle is implicitly associated with the parent's bundle group via `getReferencingBundles`. V3 Rust bundler incorrectly creates a separate bundle group for CSS, connected via `JS_bundle -> CSS_bundle_group references(4)`, which triggers runtime loading infrastructure.

**Fix needed**: In the Rust materialization, type-change bundles should be created as standalone bundle nodes (no bundle group), connected to the parent bundle via `References(4)` edge (bundle-to-bundle). The CSS bundle should be added to the same bundle group as the parent JS bundle via `bundle_group -> css_bundle Bundle(3)`.

**Status**: FIXED - Type-change bundles no longer get their own bundle group. CSS bundle is now a sibling in the parent bundle group with References(4) edge. Test passes.

---

### FAIL 4: `reuses existing async bundle instead of creating shared bundle`

**Expected**:

```
entry bundle: [index.js, esmodule-helpers.js, bundle-url.js, cacheLoader.js, js-loader.js, bundle-manifest.js]
a.js bundle: [a.js]
b.js bundle: [b.js]
c.js bundle: [c.js]
```

**Actual**:

```
entry bundle: [index.js, esmodule-helpers.js, bundle-url.js, cacheLoader.js, js-loader.js, bundle-manifest.js]  OK
a.js bundle: [a.js, bundle-url.js]  WRONG (extra bundle-url.js)
b.js bundle: [b.js, bundle-url.js]  WRONG (extra bundle-url.js)
c.js bundle: [c.js, esmodule-helpers.js]  WRONG (extra esmodule-helpers.js)
```

**Two sub-issues**:

1. `esmodule-helpers.js` in `c.js`: Confirmed from Rust (before runtimes). Same Phase 7 issue as FAIL 1 but `c.js` has multiple reaching roots (entry async-imports it, a.js/b.js sync-import it). Phase 7 fix may not cover this case.
2. `bundle-url.js` in `a.js`/`b.js`: `a.js` has a `Bundle(3)` edge to `c.js`'s bundle group because `a.js` sync-imports `c.js`. Since `c.js` has its own async bundle group (from entry's async import), the sync dep from `a.js` creates a parent Bundle(3) edge making runtimes think `a.js` needs to load `c.js`. Same root cause as Issue B.

**Status**: INVESTIGATING

---

## Common Themes / Root Issues

### Issue A: Runtime assets duplicated into async bundles (FAIL 1, 4)

`esmodule-helpers.js` is duplicated into async bundles. Investigation showed Rust serialization is correct but the issue persists. Current hypothesis: either the Rust Contains traversal is placing the asset into the async bundle, or `applyRuntimes` adds it via a code path that doesn't check `isAssetReachableFromBundle`.

### Issue B: Sync type-change/reused bundles use Bundle(3) edges instead of References(4) (FAIL 3, 4)

CONFIRMED: CSS type-change bundles and reused async bundles (sync-imported from other async bundles) are wired with `Bundle(3)` edges to their parent bundles. `JSRuntime` interprets these as async children needing JS-based loading, so it injects `bundle-url.js`.

**Fix**: In `crates/atlaspack_bundling/src/ideal_graph/mod.rs`, when materializing edges for sync type-change boundaries, use `References(4)` edge type instead of `Bundle(3)`. The ideal graph builder already detects type-change boundaries (`DecisionKind::BoundaryCreated { type_change: true }`), so this metadata needs to flow into the materialization step.

### Issue C: Async internalization missing (FAIL 2)

The Rust ideal graph builder does not internalize async bundles whose root asset is already synchronously available. This is a missing algorithm step.

---

## Notes / Context for Recovery

- The Rust ideal graph builder lives in `crates/atlaspack_bundling/src/ideal_graph/builder.rs`
- The materialization (ideal graph -> NativeBundleGraph) happens in `Bundler::bundle()` in `mod.rs` (lines ~51-466)
- The JS-side deserialization is in `BundleGraphRequestRust.ts` `getBundleGraph()` function
- `isSplittable` on bundles was hardcoded to `false` in Rust but now derived from asset graph
- The `NativeBundleGraphEdgeType` enum values: Null=1, Contains=2, Bundle=3, References=4, InternalAsync=5, Conditional=6
- These must match `bundleGraphEdgeTypes` on the JS side (defined in `packages/core/core/src/BundleGraph.ts`)
- Runtimes are applied JS-side after deserialization via `applyRuntimes()` in `packages/core/core/src/applyRuntimes.ts`
- `isAssetReachableFromBundle` in `BundleGraph.ts` (around line 1004) checks if an asset is reachable from ancestor bundles
- `applyRuntimes` merges runtimes bundle graphs via `bundleGraph.merge(runtimesBundleGraph)` (line 298 of applyRuntimes.ts)
- Non-v3 bundler tests: 73 passing, 0 failing (our changes don't break them)
