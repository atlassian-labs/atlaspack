---
'@atlaspack/optimizer-swc': minor
'@atlaspack/feature-flags': patch
---

Switch JS minification to optional OXC path behind a feature flag for significant performance gains.

- New feature flag: `useOxcMinifier` (off by default)
- When enabled, JS minification uses `oxc-minify` instead of SWC
- Benchmarks: three-js avg build time improved from ~15.5s to ~12.5s (~19%) on Apple Silicon
- Output parity: byte-identical JS and source maps verified for the three-js benchmark

This keeps SWC as the default path, enabling a safe opt-in to the faster OXC-based minification.
