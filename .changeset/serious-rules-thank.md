---
'@atlaspack/feature-flags': patch
'@atlaspack/packager-js': patch
---

Fixes scope hoisting by iterating from the main entry asset of a bundle when packaging. Behind the feature flag `applyScopeHoistingImprovementsV2`.
