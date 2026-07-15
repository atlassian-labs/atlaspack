---
'@atlaspack/transformer-compiled': major
---

Remove unused `classNameCompressionMap` and `classNameCompressionMapFilePath` options from the TypeScript compiled transformer.

These options were never used in production by any Atlassian product. They have been removed from `CompiledTransformerOpts` and the transformer config handling.

**Breaking change:** If you were setting `classNameCompressionMap` or `classNameCompressionMapFilePath` in your transformer config, remove them. These options no longer exist.
