---
'@atlaspack/transformer-compiled-external': minor
'@atlaspack/transformer-compiled': minor
'@atlaspack/transformer-js': minor
'@atlaspack/core': minor
'@atlaspack/rust': minor
---

Add new Transformer `setup` method and deprecate `loadConfig`.

Atlaspack is moving to a pure Transformer model to improve caching performance and consistency.
The old `loadConfig` method which ran once per Asset goes against this behaviour is now deprecated.
The new `setup` method runs once per Transformer instance, allowing for better caching and performance optimizations.
