---
'@atlaspack/source-map': minor
'@atlaspack/core': minor
'@atlaspack/rust': patch
---

Change approach to source map offset for hashRefs - use a streaming approach to avoid loading large sourcemaps into memory.
