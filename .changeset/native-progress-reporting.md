---
'@atlaspack/core': minor
'@atlaspack/rust': minor
'@atlaspack/types-internal': minor
'@atlaspack/utils': minor
'@atlaspack/reporter-cli': minor
---

Add native build progress reporting.

Fires `BuildProgressEvent` from Rust requests back to JS reporters via a fire-and-forget
`ThreadsafeFunction` callback. Works in both `atlaspackV3` and `fullNative` build paths.

Events:

- `building` — per-asset progress from AssetGraphRequest (completeAssets / totalAssets)
- `bundling` — once from BuildRequest before bundle graph creation
- `packagingAndOptimizing` — ready for when native packaging is wired up

Adds `BuildingProgressEvent` type and CLI reporter handling.
