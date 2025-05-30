---
'@atlaspack/feature-flags': minor
'@atlaspack/bundler-default': minor
---

Add `mergeLeastCodeLoadedSharedBundles` feature flag which enables merging a shared bundle into another shared bundle to meet maxParallelRequests when the merge leads to less code loaded compared to merging the shared bundle into the bundleGroup.
