---
'@atlaspack/core': patch
'@atlaspack/rust': patch
---

Add support for star re-exports (`export * from './dep'`) in the Rust symbol tracker.

This enables proper symbol propagation through barrel files that use star re-exports,
allowing tree-shaking to work correctly for these patterns.

Note: This does not yet handle the ambiguous re-export case where multiple star re-exports
provide the same symbol name. That case currently requires runtime namespace fallback and
will be addressed in a future update.
