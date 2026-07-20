---
'@atlaspack/transformer-js': patch
'@atlaspack/rust': patch
---

Clean up the `activate_reject_on_unresolved_imports` flag in `SYNC_DYNAMIC_IMPORT_CONFIG`.

The flag was added to safely enable rejecting promises for unresolved dynamic imports in SSR code. It is now always on, so the flag has been removed and unresolved imports always generate a rejecting promise when a config is present.

Unresolved dynamic imports now reject with an `Error` object instead of a string. As before, the rejection only fires at runtime when `globalThis.__SSR_TEMP_THROW_ON_UNRESOLVED_DYNAMIC_IMPORT` is set.
