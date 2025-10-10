---
'@atlaspack/transformer-js': patch
'@atlaspack/core': patch
'@atlaspack/rust': patch
---

We no longer want to use `meta` fields internally in Atlaspack, so they have been removed from the Rust side of the codebase, and are mapped back to the legacy format on the JS side.
