# Dynamic Nested CSS Support Plan

## Goal
Bring the SWC native transformer to feature parity with the Babel plugin when users provide nested dynamic CSS objects (selectors, pseudo selectors, and at-rules) inside `styled` objects. Today these shapes are flattened correctly in Babel but the SWC path silently drops the descendant rules, leading to missing style rules during the large-scale diff.

## Key Behaviours To Match
- Nested selectors like `'&:hover, &:focus'`, `'& > *'`, etc. must emit atomic rules for each selector combination.
- Nested `@media` / other atomic at-rules that contain dynamic declarations should also be emitted.
- Dynamic conditionals (`cond ? valueA : valueB`) inside nested blocks need runtime class conditions that mirror Babel output.
- Existing base-level behaviour (simple objects, template literals, runtime variables) must remain unchanged.

## Implementation Strategy
1. **Refactor dynamic object processing**
   - Extend `process_dynamic_css_object` to accept the current selector/at-rule context instead of always assuming the base selector.
   - Detect when a dynamic property points to an object or array to determine if it represents a selector branch or at-rule body, mirroring the static object pipeline.
2. **Handle selector nesting**
   - Use the existing `extend_selectors` helper to compose the parent selector list with the nested key.
   - Recurse into nested objects with the updated selector context.
3. **Handle at-rules**
   - When a key starts with `'@'`, convert it into an `AtRuleInput`, propagate via recursion, and merge results back into the parent artifacts.
   - Support both single objects and arrays of rule blocks.
4. **Emit declarations with context**
   - When collecting declarations for the current nesting level, wrap the declaration text with the active selectors and at-rules before passing to `css_artifacts_from_literal` so hashes match Babel output.
   - Preserve runtime variables and class conditions that arise while processing values.
5. **Preserve compatibility**
   - Keep the old code paths for template literals, existing runtime handling, and static value evaluation untouched.
   - Ensure recursion terminates gracefully on unsupported shapes by returning `None`.

## Test Coverage
- Add a regression fixture that captures the AsyncList-style pattern: nested selectors combining pseudo selectors, descendant selectors, dynamic runtime values, and conditional branches.
- Regenerate `actual.js` via the fixture generator, verify it fails before the fix, and then confirm it passes afterwards.
- Re-run `cargo test -p compiled_swc_plugin` fixtures and `node jira/analyze-style-rules.js` (after regenerating the specific SWC `style-rules.json`) to ensure real-world parity.

## Follow-up Validation
1. Regenerate the SWC style rules for `AsyncList.tsx` using `collect_style_rules --only`.
2. Execute `jira/analyze-style-rules.js` until no behavioural mismatches remain.
3. Document any remaining cosmetic differences (if any) per `AGENTS.md`.
