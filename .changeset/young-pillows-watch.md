---
'@atlaspack/types-internal': minor
'@atlaspack/feature-flags': minor
'@atlaspack/bundler-default': minor
'@atlaspack/transformer-js': minor
'@atlaspack/packager-js': minor
'@atlaspack/runtime-js': minor
'@atlaspack/core': minor
---

Add support for bundle merging based on `webpackChunkName` comments.

Adding a `webpackChunkName` comment to an import will allow the bundler to merge multiple imports into a single bundle.

e.g.:

```ts
import(/* webpackChunkName: "my-chunk" */ './my-module');
import(/* webpackChunkName: "my-chunk" */ './another-module');
```

This can be enabled with the feature flag `supportWebpackChunkName`.
