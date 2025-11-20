---
'@atlaspack/rust': patch
---

Add an `on_new_build` hook to the ResolverPlugin trait, which allows us to clear the resolver cache between builds. This allows for the discovery of previously non-existent assets on the next incremental build.
