---
'@atlaspack/utils': patch
---

No longer bundles `@atlaspack/feature-flags` into the source, so it will work from the same feature flag state as the rest of the repo.

Previously, this was preventing us from using feature flags inside the utils package, as it was impossible to set them.
