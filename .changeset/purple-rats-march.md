---
'@atlaspack/core': minor
'@atlaspack/transformer-js': patch
'@atlaspack/transformer-babel': patch
'@atlaspack/feature-flags': patch
---

feat(core, transformers): add feature-flag to omit sourcesContent from memory; reduce peak memory during builds

- Introduce `omitSourcesContentInMemory` feature flag to stop retaining full source text in `sourcesContent` throughout transforms. Default OFF; behavior unchanged unless enabled.
- Guard `asset.sourceContent` initialization and `setSourceContent`/`sourcesContent` copies behind the flag.
- Mappings and source paths remain correct; packager still inlines or references sources per config.

Memory (three-js benchmark, V3, 1 run):

- Baseline OFF: later, larger compactions near end of build (e.g. `~44.2s Mark-Compact 20.4 (50.2) -> 12.5 (53.5) MB`).
- Flag ON: earlier compactions during transform/packaging, keeping old space ≈10–11 MB (e.g. `~17.7s Mark-Compact 11.5 (28.0) -> 9.6 (27.5) MB`).

Sourcemaps: unchanged by default; with flag ON, only the in-memory retention is removed.
