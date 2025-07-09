---
'@atlaspack/feature-flags': patch
'@atlaspack/core': patch
'@atlaspack/rust': patch
---

Fixes an issue where star re-exports of empty files (usually occurring in compiled typescript libraries) could cause exports to undefined at runtime.
Fix is behind the feature-flag `emptyFileStarRexportFix`.
