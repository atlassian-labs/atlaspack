---
'@atlaspack/core': patch
'@atlaspack/rust': patch
---

Fix native config loader rejecting TypeScript entry points in package.json.

The Rust package.json deserializer rejected `.ts` and `.tsx` extensions in builtin target fields
(`main`, `browser`, `module`), causing builds to fail with "Unexpected file type" errors when a
package.json uses TypeScript source entry points (e.g. `"main": "index.ts"`).

Additionally fixes EntryRequest to resolve the package path using `cwd()` instead of `project_root`,
matching the JS-side behavior for correct target resolution in monorepo setups.
