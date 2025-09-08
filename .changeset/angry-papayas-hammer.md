---
'@atlaspack/feature-flags': patch
'@atlaspack/transformer-css': patch
---

Add new feature flag `preserveUnstableSingleFileOutputInCss` which when enabled will ensure the `unstableSingleFileOutput` property on the asset environment is preserved when transforming CSS.
