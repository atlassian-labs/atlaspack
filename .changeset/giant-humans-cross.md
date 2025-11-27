---
'@atlaspack/feature-flags': patch
'@atlaspack/rust': patch
---

Disallows circular package aliases during resolution. This fixes cases where package A aliases to package B, but package B imports package A, causing infinite loops during resolution. Instead, we disable the alias allow package B to import the real package A.

This is useful in cases where you create wrapper packages that re-export another package under a different name.

Enabled via the `disallowCircularPackageAliases` feature flag.
