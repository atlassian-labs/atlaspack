---
'@atlaspack/core': patch
'@atlaspack/rust': patch
'@atlaspack/feature-flags': patch
---

Add native end-to-end build pipeline via BuildRequest.

When the `fullNative` feature flag is enabled, the entire build pipeline (asset graph, bundle graph,
packaging) runs natively in Rust via a single NAPI call, bypassing the JS request tracker.

Key changes:

- Add `BuildRequest` composing `AssetGraphRequest` and `BundleGraphRequest` with a packaging stub
- Add `Atlaspack::build()` method and `atlaspack_napi_build` NAPI binding
- Add `fullNative` feature flag gating the native path in `Atlaspack.ts._build()`
- Packaging step is a no-op pending PackagingRequest implementation
