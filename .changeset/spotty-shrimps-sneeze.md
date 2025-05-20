---
'@atlaspack/apvm': patch
---

Yarn install resets permissions on binaries. This change checks the binary has the correct permissions before running it and if not, sets the correct permissions
