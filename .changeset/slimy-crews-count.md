---
'@atlaspack/bundler-default': patch
---

In the first attempt to support isolated bundles, there was a check on the number of assets that wasn't really correct.
That check has been removed, so we can bundle even where there are special cases.
