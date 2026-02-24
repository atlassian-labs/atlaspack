---
'@atlaspack/rust': patch
---

Add support for namespace re-exports (`export * as ns from './dep'`) in the Rust symbol tracker.

This enables proper symbol propagation through barrel files that use namespace re-exports,
allowing tree-shaking to work correctly for these patterns. Includes support for:

- Basic namespace re-exports
- Namespace re-exports alongside named exports
- Namespace re-exports alongside star re-exports
- Multiple namespace re-exports from the same barrel
- Chained namespace re-exports (multiple levels deep)
