---
'@atlaspack/transformer-compiled-css-in-js': major
'@atlaspack/rust': major
---

Remove unused `classNameCompressionMap` option and `ac` runtime helper.

The `classNameCompressionMap` option was never used in production by any Atlassian product. It has been removed alongside the `ac()` runtime helper and the related `compress-class-names-for-runtime` and `get-runtime-class-name-library` utilities.

All class name merging now unconditionally uses `ax()`.

**Breaking change:** If you were setting `classNameCompressionMap` in your transformer config, remove it. The option no longer exists.
