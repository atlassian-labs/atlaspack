---
'@atlaspack/core': patch
'@atlaspack/rust': patch
---

Update some Rust internals to use Arcs instead of passing references. This won't make any difference in the immediate term, but is required setup for the next set of changes
