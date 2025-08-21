---
'@atlaspack/feature-flags': patch
'@atlaspack/bundler-default': patch
---

Adds a new feature flag `singleFileOutputStableName` - when enabled, bundles produced by the experimental single file output bundler will have stable names (i.e. no hash).
