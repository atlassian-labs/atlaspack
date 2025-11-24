---
'@atlaspack/core': patch
'@atlaspack/rust': patch
---

Makes the serialization and LMDB write steps into separate Promises, so that we can return them separately and parallelise some work.
