---
'@atlaspack/feature-flags': patch
'@atlaspack/packager-js': patch
---

Added a new feature flag `inlineConstOptimisationFix` which when enabled changes the behaviour for output of constant modules. This fixes two issues with constant modules:

- Previously constant modules, if they needed a namespace anywhere, would have a namespace everywhere, with this change they only have a namespace in the bundles where needed.
- Previously in the case of wrapped assets, a constant module dependnecy of that wrapped asset would be rendered after the module - which meant the minifier would not be able to inline the constants safely. With this flag all constant modules are rendered at the top of the bundle.
