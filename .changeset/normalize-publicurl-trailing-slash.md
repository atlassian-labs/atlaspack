---
'@atlaspack/cli': patch
'@atlaspack/feature-flags': patch
'@atlaspack/integration-tests': patch
---

Add feature flag to normalize publicUrl trailing slash in dev server

This change adds a new feature flag `normalizePublicUrlTrailingSlash` that, when enabled, automatically adds a trailing slash to `publicUrl` values in the dev server. This prevents issues where URLs without trailing slashes (e.g., `http://localhost:8080` or `/assets`) could result in malformed asset URLs like `localhost:8080assets.json` instead of `localhost:8080/assets.json`.

The feature flag is disabled by default to maintain backward compatibility. Enable it with:

```js
featureFlags: {
  normalizePublicUrlTrailingSlash: true;
}
```
