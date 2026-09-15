---
'@atlaspack/transformer-compiled-css-in-js': minor
'@atlaspack/rust': minor
---

Add an opt-in `collisionResistantHash` option to the Compiled CSS-in-JS transformer.

When enabled, atomic class names are generated using a base-62 fixed-width hash (`_<6-char group><4-char value>`, 11 chars) instead of the legacy base-36 truncated hash (`_<4><4>`, 9 chars). This eliminates the group-hash collisions that can cause `ax()` to incorrectly de-duplicate unrelated declarations.

The option defaults to `false`, so output is byte-for-byte unchanged unless it is explicitly enabled. The two formats are length-disjoint, so legacy (9-char) and new (11-char) classes can safely co-exist on the same page during migration. The base-62 hash matches the reference implementation in `@compiled/css`.
