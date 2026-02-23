---
'@atlaspack/core': patch
'@atlaspack/rust': patch
---

Add support for star re-exports (`export * from './dep'`) in the Rust symbol tracker.

This enables proper symbol propagation through barrel files that use star re-exports,
allowing tree-shaking to work correctly for these patterns. Includes support for:

- Chained star re-exports (multiple levels of `export *`)
- Diamond patterns (multiple paths to the same symbol through different star re-exports)
- Speculative requirement cleanup (efficient tracking and removal of unsatisfied paths)

Note: This does not yet handle the ambiguous re-export case where multiple star re-exports
provide the same symbol name. That case currently requires runtime namespace fallback and
will be addressed in a future update.
