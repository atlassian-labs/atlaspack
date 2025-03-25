---
'@atlaspack/core': minor
---

`loadPlugin` no longer returns a `range` field. This field was only set by looking up a package's `parcelDependencies`, which no longer exist.
