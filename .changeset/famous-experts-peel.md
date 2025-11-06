---
'@atlaspack/feature-flags': patch
'@atlaspack/transformer-js': patch
'@atlaspack/rust': patch
---

Fix issue where nested Promise.resolve calls mixed with dynamic imports could cause build errors
