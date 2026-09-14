---
'@atlaspack/bundler-default': patch
---

Prevent self-references when pruning cyclic reused bundles to satisfy the parallel request limit, avoiding missing-edge build failures.
