---
'@atlaspack/build-cache': patch
'@atlaspack/feature-flags': patch
---

Add LargeMap to work around Node 24's Map size limit in build cache serializer.

This change is behind the `useLargeMapInBuildCache` feature flag.
