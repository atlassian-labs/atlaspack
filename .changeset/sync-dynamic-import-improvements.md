---
'@atlaspack/transformer-js': minor
'@atlaspack/rust': minor
'@atlaspack/feature-flags': minor
---

sync_dynamic_import: Remove `activate_reject_on_unresolved_imports` feature gate (now always enabled). Add `sync_require_paths` config with glob matching that wraps matched imports in `Promise.resolve(require(...))` instead of bare `require()`. Add `syncDynamicImportRejectWithError` feature flag that, when enabled, causes unresolved dynamic imports to reject with `new Error(message)` (with `.skipSsr = true`) instead of a plain string.
