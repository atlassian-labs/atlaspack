---
'@atlaspack/packager-js': patch
---

Fix the scope hoisting improvements feature by making all entry assets of a bundle
wrapped, which means there is no top level scope.
