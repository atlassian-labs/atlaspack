---
'@atlaspack/core': minor
---

Add a step to the BundleGraphRequest that will scan for assets that have a transitive dep on `@confluence/loadable` and marks them as having side effects.
This allows the inline requires optimizer to be applied to projects that don't necessarily declare side effects correctly.
