---
'@atlaspack/feature-flags': patch
'@atlaspack/core': patch
'@atlaspack/packager-js': patch
'@atlaspack/runtime-js': patch
'@atlaspack/runtime-react-refresh': patch
'@atlaspack/transformer-js': patch
'@atlaspack/transformer-react-refresh-wrap': patch
'@atlaspack/rust': patch
---

Remove the hmrImprovements feature flag while preserving its disabled behavior, including the separate React Refresh runtime and legacy refresh helper.
