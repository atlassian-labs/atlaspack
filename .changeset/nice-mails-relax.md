---
'@atlaspack/rust': patch
'@atlaspack/transformer-js': minor
---

Add `activate_reject_on_unresolved_imports` config option to `sync-dynamic-import` transformer to create a rejecting promise for unresolved imports if `globalThis.__SSR_TEMP_THROW_ON_UNRESOLVED_DYNAMIC_IMPORT` is set.
