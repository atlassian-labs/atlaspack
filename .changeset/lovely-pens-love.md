---
'@atlaspack/types-internal': minor
'@atlaspack/feature-flags': minor
'@atlaspack/packager-js': minor
'@atlaspack/core': minor
---

Introduce a new `getReferencedAssets(bundle)` method to the BundleGraph to pre-compute referenced assets, this is used by the scope hoisting packager behind a new `precomputeReferencedAssets` feature flag.
