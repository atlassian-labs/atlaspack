---
'@atlaspack/bundler-default': minor
---

Add `sharedBundleMergeThreshold` config option

In apps with lots of dynamic imports, many shared bundles are often removed
from the output to prevent an overload in network requests according to the
`maxParallelRequests` config. In these cases, setting `sharedBundleMergeThreshold` can
merge shared bundles with a high overlap in their source bundles (bundles that share the bundle).
This config trades-off potential overfetching to reduce asset duplication.

The following config would merge shared bundles that have a 75% or higher overlap in source bundles.

```json
{
  "@atlaspack/bundler-default": {
    "sharedBundleMergeThreshold": 0.75
  }
}
```
