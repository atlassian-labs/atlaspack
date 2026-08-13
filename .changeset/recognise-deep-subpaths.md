---
'@atlaspack/transformer-js': patch
'@atlaspack/rust': patch
---

Recognise deep subpath imports of `@atlassian/react-async` (e.g. `@atlassian/react-async/js-resource-for-user-visible`) in the React Async import-lifting transform.

Previously the transform only matched the bare `@atlassian/react-async` specifier, so `JSResourceForUserVisible` calls imported via a tree-shaking-friendly deep subpath silently skipped SSR import lifting.
