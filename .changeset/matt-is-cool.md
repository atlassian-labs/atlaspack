---
'@atlaspack/transformer-js': patch
'@atlaspack/core': patch
'@atlaspack/feature-flags': patch
'@atlaspack/rust': patch
---

Fix scope hoisting crash when a module has both `import * as ns from 'x'` and `export * from 'x'` for the same specifier. The wildcard re-export was overwriting the namespace import binding on the shared dependency, causing a ReferenceError in the bundled output. Gated behind `fixExportStarNamespaceOverwrite` feature flag.
