---
'@atlaspack/transformer-js': patch
'@atlaspack/rust': patch
---

Fix (behind a feature flag) the bug where non-static property access of an imported object was not being considered used by the collector.
