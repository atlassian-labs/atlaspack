---
'@atlaspack/core': patch
---

Removes the dependency check within the config default `package.json`.

Any dependencies that used to be auto-installed from `parcelDependencies` should
now be installed in the project root.
