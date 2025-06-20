---
'@atlaspack/feature-flags': patch
'@atlaspack/packager-js': patch
---

Added a new feature flag `inlineConstNamespaceFix` which when enabled changes the behaviour for output of constant modules. Previously constant modules if they needed a namespace anywhere would have a namespace everywhere, with this change they only have a namespace in the bundles where needed.
