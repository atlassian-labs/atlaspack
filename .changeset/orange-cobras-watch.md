---
'@atlaspack/bundler-default': minor
---

Adds additional support for asset types when using the `singleFileOutput` option.

When the single file output option was first defined, it was a very quick and naive
implementation that only added JS assets to a single bundle, primarily to support
SSR runtimes that only allow a single file.

This falls apart when attempting to server render something like an SVG, because
the existing implementation would ignore them entirely.

This is fixed by adding support for "isolated" bundles, of which SVGs are one but
not the only use case.
The content itself is not included in the primary bundle, but the references between
them (like an SVG URL) will now be inserted.
