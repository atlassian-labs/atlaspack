---
'@atlaspack/integration-tests': minor
'@atlaspack/feature-flags': minor
'@atlaspack/rust': minor
---

Added logic to rust JS transformer to account for JSX transformations in scenarios when the file extension of the asset is NOT .jsx or .tsx. The logic to determine whether the file should be treated as JSX syntax now mirrors that of the existing v2 behaviour. Several unit tests and integration tests have been introduced to ensure this parity is maintained.
