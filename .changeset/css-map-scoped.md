---
'@atlaspack/transformer-js': minor
---

Add `cssMapScoped` API to the Rust/SWC pipeline — a dedicated, non-atomic variant of `cssMap`.

`cssMapScoped` is a sibling API to `cssMap` that emits a **single non-atomic class per variant** (e.g. `cc-d76ndi`), rather than one atomic class per CSS declaration (e.g. `_1234abcd _5678efgh ...`). It is intended for use cases where downstream consumers need to override styles via descendant selectors (e.g. editor / toolbar code).

### Usage

```ts
import {cssMapScoped} from '@compiled/react';

const styles = cssMapScoped({
  panelStyles: {
    '.editor .panel': {padding: '8px', backgroundColor: 'blue'},
    '.editor .panel-title': {fontWeight: 'bold', color: 'blue'},
  },
  dangerStyles: {
    '.editor .panel': {backgroundColor: 'pink'},
    '.editor .panel-title': {color: 'red'},
  },
});
// → { panelStyles: 'cc-d76ndi', dangerStyles: 'cc-a6u7cm' }
```

### `atlassian-swc-compiled-css` (Rust crate)

- **New `cssMapScoped` API recognised** alongside `cssMap` in `babel-plugin.rs`. `cssMapScoped` imports are tracked separately via the new `CompiledImports.css_map_scoped: Vec<String>` field in `types.rs`, allowing the visitor to distinguish atomic vs non-atomic calls at the call site without needing an extra options argument.
- **New `CssMapKind` enum** (`Atomic` / `NonAtomic`) added to `css-map/index.rs`, replacing ad-hoc boolean flags. Threaded through `visit_css_map_path` / `visit_css_map_path_with_builder` to make the transform mode explicit.
- **New `non_atomicify_rules_plugin`** (new file `postcss/plugins/non-atomicify-rules.rs`) mirrors `atomicify_rules_plugin` structurally. Scopes all CSS declarations of a variant under a single pre-computed `.cc-<hash>` class. Reuses these helpers from `atomicify-rules` (newly exposed as `pub(super)`): `replace_nesting_selector`, `normalize_selector`, `collect_rule_selectors`, `parse_selector_as_rule`, `can_atomicify_at_rule`, `is_comment_list`. Registered in `postcss/plugins/mod.rs`.
- **PostCSS pipeline non-atomic path** added in `postcss/postcss_pipeline.rs` — when `TransformCssOptions.atomic == Some(false)`, the pipeline runs `non_atomicify_rules_plugin` (after autoprefixer + normalise) and emits sheets with the pre-computed class. `postcss/transform.rs` gains `atomic: Option<bool>` and `non_atomic_class_name: Option<String>` fields on `TransformCssOptions`.
- **Non-atomic class name** computed at compile time as `cc-{hash(filename:binding:variantKey)}` (prefix from new `NON_ATOMIC_CLASS_PREFIX` constant in `constants.rs`) — including the binding name prevents collisions when two `cssMapScoped` calls in the same file share a variant key (e.g. `panelStyles.default` vs `dangerStyles.default`).
- **Plumbing**: `utils/transform-css-items.rs` gains a new `TransformCssItemsOptions` struct (with `atomic` and `non_atomic_class_name` fields, defaulting to atomic mode for backward compatibility). All existing callers (`utils/build-compiled-component.rs`, `utils/build-styled-component.rs`, `utils/css-builders.rs`, `class-names/index.rs`, `xcss-prop/index.rs`) pass `&TransformCssItemsOptions::default()` for atomic-mode behaviour.
- **Comprehensive coverage**: `&:hover` / `&:focus` parent-pseudo flattening, `& descendant` (inside outer key), right-side `&` for RTL (`[dir="rtl"] &`), comma-separated selectors (`.a, .b`), nested `@media` + `@supports`, `@keyframes` step keywords correctly NOT scoped, vendor prefixes via autoprefixer, and `css={[base, override1, override2]}` array stacking.

### E2E tests

Two new e2e fixtures under `packages/core/e2e-tests/test/data/`:

- `simple-project-with-css-map-scoped-extracted/` — `extract: true`, verifies extracted CSS is linked from `<head>` and applies in the browser.
- `simple-project-with-css-map-scoped-runtime/` — `extract: false`, verifies CSS is hoisted as `const _N = "..."` strings injected at runtime by the `CC` / `CS` wrapper.

Both fixtures exercise nested descendant selectors, `& .panel-icon` flattening, RTL right-side ampersand, `@keyframes` animations, `@media` queries, nested `@supports` + `@media`, override stacking via `css={[...]}`, and vendor prefixes.

Two new e2e tests in `packages/core/e2e-tests/test/compiled-css-in-js.test.mts` (driven by shared `assertCssMapScopedFixture()` helper) assert the rendered DOM matches expected behaviour for both modes.

### Developer experience

- **Readable e2e dist directory names** — `packages/core/e2e-tests/utils/build-fixture.mts` now derives the output sub-directory from the fixture's parent directory name (e.g. `dist/simple-project-with-css-map-scoped-extracted/` instead of `dist/<64-char-hash>/`), with a short hash fallback to retain cache-busting safety for non-standard targets. Easier to inspect e2e output during development.

### Testing infrastructure

- Adopted **inline `insta` snapshot tests** for cssMapScoped's postcss-plugin and babel-plugin tests where the assertions are about CSS output shape. Uses a `normalize_cc_hashes()` helper to map dynamic `cc-<hash>` class names to a stable `cc-xxxxxx` placeholder so snapshots don't depend on hash inputs.
- Added `*.pending-snap` to `.gitignore` (insta scratch files should never be committed).
