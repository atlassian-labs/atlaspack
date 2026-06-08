---
'@atlaspack/transformer-js': patch
---

Add experimental `hashStrategy` option to `cssMap` for **internal use only** in the Rust/SWC pipeline.

`hashStrategy` is **not officially supported** and may change or be removed without notice. It is intentionally not exposed in public TypeScript types, and can only be used with extreme caution.

### `atlassian-swc-compiled-css` (Rust crate)

- Added `HashStrategy` enum (`Default`, `Enhanced`, `Max`) to `atomicify-rules`.
- Added `hash_base62` function to `utils/hash` — MurmurHash2 encoded in base-62 (0-9, a-z, A-Z), providing 8.8× more combinations per character than base-36.
- `atomic_class_name` now branches on `hash_strategy`:
  - `Default` — original base-36, 4-char group → 9-char class (unchanged behaviour).
  - `Enhanced` — base-62, 4-char group → 9-char class, reduced collision risk.
  - `Max` — base-62, 6-char group → 11-char class, structurally incompatible with default/enhanced.
- `cssMap` now accepts an optional second argument `{ hashStrategy: 'default' | 'enhanced' | 'max' }`, validated at compile time with clear error messages for unknown options or invalid strategy values.
- `hash_strategy` is threaded through `transform_css_items` → `transform_css_item` → `create_transform_css_options` → `TransformCssOptions` → `atomicify-rules`.
