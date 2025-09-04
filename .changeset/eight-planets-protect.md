---
'@atlaspack/core': patch
'@atlaspack/rust': patch
---

Guard against empty buffers being returned from JS workers, when using the V3 asset graph build
