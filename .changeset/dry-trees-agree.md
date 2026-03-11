---
'@atlaspack/rust': patch
'@atlaspack/transformer-js': patch
---

Improve SWC compiled CSS-in-JS browserslist resolution, behaviour correctness, and diagnostics.

- Add `browserslistEnv` config option so autoprefixer and cssnano plugins resolve
  the correct browserslist environment (e.g. "development" vs "production"),
  matching Babel's behavior.
- Cache resolved `@compiled/css` package path per-process to avoid repeated
  filesystem walks. This is required to align with postcss browserlist resolution behaviour.
- Detect sheet identifier collisions with existing module bindings and emit
  diagnostics to prevent "sheet.includes is not a function" runtime errors.
- Validate sheet content is a non-empty CSS rule string.
- Add vendor autoprefixer support for mask-composite, appearance, and
  general selector pseudo-class variants (e.g. :read-only, :read-write).
- Add cssnano normalize plugins: discard-comments, minify-gradients,
  normalize-timing-functions, and calc reduction.
