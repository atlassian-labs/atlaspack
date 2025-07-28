---
'@atlaspack/node-resolver-core': patch
'@atlaspack/feature-flags': patch
---

Fixes the handling of node builtins in library mode.

Previously, the function to check whether a node builtin should be external didn't account
for the case where `includeNodeModules` was set to just `true`.

This caused library builds to include require statements to node builtins in the bundle,
even when targeting a browser environment.

Enabling the `libraryBuiltinsFix` feature flag will fix this issue.
