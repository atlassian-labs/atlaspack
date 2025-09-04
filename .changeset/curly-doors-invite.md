---
'@atlaspack/rust': patch
---

Added logic to rust JS transformer to account for JSX transformations in scenarios when the file extension of the asset is NOT .jsx or .tsx. The logic to determine whether the file should be treated as JSX syntax now much more closely resembles the existing v2 behaviour.
