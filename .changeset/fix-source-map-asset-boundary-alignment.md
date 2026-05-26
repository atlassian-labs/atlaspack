---
'@atlaspack/packager-js': patch
'@atlaspack/optimizer-inline-requires': patch
'@atlaspack/optimizer-swc': patch
'@atlaspack/rust': patch
---

Fix misaligned JavaScript bundle source maps.

Three related fixes that together resolve the long-standing source-map
misalignment in atlaspack-produced JS bundles (where positions near asset
boundaries were attributed to the _previous_ asset's source file):

1. `@atlaspack/packager-js`: emit an asset-boundary mapping at the start of
   every asset's region in the bundle. Without this anchor, source-map
   consumers (and downstream optimizers that compose maps via
   nearest-neighbour lookup) attributed columns at the start of an asset to
   whichever mapping the previous asset emitted last — especially visible
   for codegen'd files such as `__generated__/*.graphql.ts` whose own maps
   are sparse near column 0. Also fixes an off-by-N drift in
   `getHoistedParcelRequires` where `lineCount` was incremented by the
   unfiltered count when emitting filtered hoisted values.

2. `@atlaspack/optimizer-inline-requires` + `@atlaspack/rust`: forward the
   bundle's input source map directly to swc via swc's native
   `inputSourceMap` parameter (plumbed through a new
   `input_source_map: Option<String>` field on
   `InlineRequiresOptimizerInput`). The previous flow returned a map rooted
   at `<anon>` and composed it in JavaScript using
   `SourceMap.extends`, which snaps unmatched positions to the nearest
   mapping and produced incorrect attribution across asset boundaries.

3. `@atlaspack/optimizer-swc`: the SWC optimizer now passes the bundle's
   input source map to swc directly (instead of letting
   `SourceMap.extends` post-compose). This removes the residual `<anon>`
   sources from the final map and improves alignment for tokens that swc
   does not emit a mapping for.

Measured improvement on Confluence production builds: string-literal
alignment on the largest bundles goes from ~34% (pre-fix) to 54–97% (post-fix),
with `<anon>` sources eliminated entirely.
