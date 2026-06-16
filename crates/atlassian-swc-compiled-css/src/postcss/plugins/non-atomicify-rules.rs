//! Non-atomic CSS scoping plugin — Rust port of `packages/css/src/plugins/non-atomicify-rules.ts`.
//!
//! Unlike `atomicify-rules` (which splits each declaration into its own `._hash { decl }` rule),
//! this plugin scopes **all** declarations under a single `.cc-<hash>` class, preserving the
//! grouped structure. This is used by `cssMapScoped` to produce one class per variant instead
//! of one class per CSS property.
//!
//! ## Differences from `atomicify-rules`:
//!
//! | Situation              | atomicify-rules              | non-atomicify-rules                |
//! |------------------------|------------------------------|------------------------------------|
//! | `QualifiedRule`        | split → N atomic rules       | replace selector → 1 scoped rule   |
//! | `ListOfComponentValues`| one atomic rule per decl     | group all decls → 1 scoped rule    |
//! | scopeable `@rule`      | recurse atomically           | recurse, keep grouped              |
//! | passthrough `@rule`    | pass through unchanged       | pass through unchanged (same)      |
//!
//! ## Reuse from `atomicify-rules`:
//!
//! The at-rule classification (`can_atomicify_at_rule`), selector collection
//! (`collect_rule_selectors`), selector parsing (`parse_selector_as_rule`),
//! selector normalisation (`normalize_selector`, `replace_nesting_selector`),
//! comment-list detection (`is_comment_list`), and serialisation helpers
//! (`serialize_node`, `serialize_component_values`) are all reused directly
//! from `atomicify_rules` (exposed as `pub(super)`).

use swc_core::common::DUMMY_SP;
use swc_core::css::ast::{
  AtRule, ComponentValue, Delimiter, DelimiterValue, QualifiedRule, Rule, Stylesheet,
};

use super::super::transform::{Plugin, TransformContext};
use super::atomicify_rules::{
  can_atomicify_at_rule, collect_rule_selectors, is_comment_list, normalize_selector,
  parse_selector_as_rule, replace_nesting_selector,
};

// ---------------------------------------------------------------------------
// Plugin struct
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone)]
pub struct NonAtomicifyRules {
  pub class_name: String,
}

impl NonAtomicifyRules {
  pub fn new(class_name: impl Into<String>) -> Self {
    Self {
      class_name: class_name.into(),
    }
  }
}

pub fn non_atomicify_rules(class_name: impl Into<String>) -> NonAtomicifyRules {
  NonAtomicifyRules::new(class_name)
}

// ---------------------------------------------------------------------------
// Plugin implementation
// ---------------------------------------------------------------------------

impl Plugin for NonAtomicifyRules {
  fn name(&self) -> &'static str {
    "non-atomicify-rules"
  }

  fn run(&self, stylesheet: &mut Stylesheet, ctx: &mut TransformContext<'_>) {
    let class_name = &self.class_name;
    let placeholder = ctx.options.declaration_placeholder.as_deref().unwrap_or("");

    let mut new_rules: Vec<Rule> = Vec::with_capacity(stylesheet.rules.len());

    for rule in std::mem::take(&mut stylesheet.rules) {
      match rule {
        Rule::QualifiedRule(qr) => {
          if let Some(scoped) = non_atomicify_qualified_rule(*qr, class_name, placeholder) {
            new_rules.push(Rule::QualifiedRule(Box::new(scoped)));
          }
        }

        Rule::ListOfComponentValues(list) => {
          // Bare declarations at root level (SWC's representation of top-level decls).
          // Group all declarations into a single scoped rule.
          if !is_comment_list(&list) {
            let decls: Vec<ComponentValue> = list
              .children
              .into_iter()
              .filter(|cv| matches!(cv, ComponentValue::Declaration(_)))
              .collect();
            if !decls.is_empty() {
              new_rules.push(Rule::QualifiedRule(Box::new(make_scoped_rule(
                class_name, decls,
              ))));
            }
          }
        }

        Rule::AtRule(at_rule) => {
          if can_atomicify_at_rule(&at_rule) {
            let scoped = non_atomicify_at_rule(*at_rule, class_name, placeholder);
            new_rules.push(Rule::AtRule(Box::new(scoped)));
          } else {
            // passthrough (@keyframes, @font-face, @property …) — unchanged
            new_rules.push(Rule::AtRule(at_rule));
          }
        }
      }
    }

    stylesheet.rules = new_rules;

    // Register the class name so `ctx.finish()` includes it in `class_names`.
    ctx.push_class_name(class_name.clone());
  }
}

// ---------------------------------------------------------------------------
// Core helpers
// ---------------------------------------------------------------------------

/// Replace the selector of a `QualifiedRule` with a scoped `.class_name` selector,
/// mirroring `scopeRule` from `non-atomicify-rules.ts`.
///
/// - If the rule's selector is the declaration placeholder or `*`, it is replaced
///   directly with `.class_name` (no nesting).
/// - Otherwise the selector is scoped: `& .child` → `.class_name .child`,
///   `&:hover` → `.class_name:hover`, etc.
///
/// Returns `None` if the rule has no declarations (empty rules are dropped).
fn non_atomicify_qualified_rule(
  mut rule: QualifiedRule,
  class_name: &str,
  placeholder: &str,
) -> Option<QualifiedRule> {
  // Collect raw selector strings (reusing atomicify-rules helper).
  let selectors = collect_rule_selectors(&rule);

  // Determine the scoped selector.
  // For each selector in the list we apply `scope_selector`; for non-atomic we
  // always produce a single combined selector (multiple selectors in the source
  // are unusual for cssMap variants but we handle them gracefully).
  let scoped_selector = selectors
    .into_iter()
    .map(|s| scope_selector(&s, class_name, placeholder))
    .collect::<Vec<_>>()
    .join(", ");

  // Replace just the prelude (selector) — reuse the existing block with its real spans.
  // Same pattern as atomicify-rules: parse a template rule to get a valid prelude,
  // then swap in the existing block value.
  let template = parse_selector_as_rule(&scoped_selector);
  rule.prelude = template.prelude;

  // Re-interleave semicolons between adjacent declarations so the SWC codegen
  // emits valid CSS (without this, `color:red` and `font-weight:bold` run together).
  let old_value = std::mem::take(&mut rule.block.value);
  rule.block.value = interleave_semicolons(old_value);
  // Note: rule.block.span and rule.block.name retain their original parsed spans — no DUMMY_SP needed.

  // Drop rules with no declarations (mirrors TypeScript's `rule.remove()` for empty nodes).
  let has_declarations = rule
    .block
    .value
    .iter()
    .any(|cv| matches!(cv, ComponentValue::Declaration(_)));
  if has_declarations { Some(rule) } else { None }
}

/// Recurse into a scopeable at-rule (`@media`, `@supports`, `@container` …),
/// scoping inner qualified rules under `.class_name`.
///
/// Mirrors `scopeAtRule` from `non-atomicify-rules.ts`.
fn non_atomicify_at_rule(mut at_rule: AtRule, class_name: &str, placeholder: &str) -> AtRule {
  if let Some(block) = at_rule.block.as_mut() {
    let old_values = std::mem::take(&mut block.value);
    let mut new_values: Vec<ComponentValue> = Vec::with_capacity(old_values.len());

    for cv in old_values {
      match cv {
        ComponentValue::QualifiedRule(inner_qr) => {
          if let Some(scoped) = non_atomicify_qualified_rule(*inner_qr, class_name, placeholder) {
            new_values.push(ComponentValue::QualifiedRule(Box::new(scoped)));
          }
        }
        ComponentValue::AtRule(inner_at) => {
          if can_atomicify_at_rule(&inner_at) {
            let scoped = non_atomicify_at_rule(*inner_at, class_name, placeholder);
            new_values.push(ComponentValue::AtRule(Box::new(scoped)));
          } else {
            new_values.push(ComponentValue::AtRule(inner_at));
          }
        }
        ComponentValue::Declaration(decl) => {
          // Bare decl inside an at-rule block — wrap it.
          new_values.push(ComponentValue::QualifiedRule(Box::new(make_scoped_rule(
            class_name,
            vec![ComponentValue::Declaration(decl)],
          ))));
        }
        other => new_values.push(other),
      }
    }

    block.value = new_values;
  }
  at_rule
}

/// Build a new `QualifiedRule` that scopes `declarations` under `.class_name`,
/// mirroring `scopeDecl` from `non-atomicify-rules.ts`.
///
/// Uses `parse_selector_as_rule` (same pattern as `atomicify-rules`) so the rule's
/// spans come from the SWC parser rather than `DUMMY_SP`.
fn make_scoped_rule(class_name: &str, declarations: Vec<ComponentValue>) -> QualifiedRule {
  let mut rule = parse_selector_as_rule(&format!(".{}", class_name));
  rule.block.value = interleave_semicolons(declarations);
  rule
}

/// Scope a raw selector string under `.class_name`.
///
/// Mirrors `normalizeSelector` + `scopeSelector` from `non-atomicify-rules.ts`:
/// - placeholder / `*` / `""` / `"&"` → `.class_name` directly
/// - selector without `&`            → `.class_name <selector>`
/// - selector with `&`               → replace `&` with `.class_name`
fn scope_selector(raw: &str, class_name: &str, placeholder: &str) -> String {
  let trimmed = raw.trim();

  // Treat placeholder, universal selector, empty string, and bare `&` as "self".
  let is_self = trimmed.is_empty()
    || trimmed == "&"
    || trimmed == "*"
    || (!placeholder.is_empty() && trimmed == placeholder);

  if is_self {
    return format!(".{}", class_name);
  }

  // Re-use atomicify-rules' normalize_selector to add a leading `&` if missing,
  // then replace_nesting_selector to substitute `&` → `.class_name`.
  let normalised = normalize_selector(trimmed);
  replace_nesting_selector(&normalised, class_name)
}

/// Insert `ComponentValue::Delimiter(Semicolon)` between adjacent
/// `ComponentValue::Declaration` nodes.
///
/// The SWC CSS codegen does **not** auto-insert semicolons between adjacent
/// `Declaration` nodes in a `SimpleBlock`, causing invalid output like
/// `color:redfont-weight:bold`.  Inserting explicit `Delimiter(Semicolon)` tokens
/// fixes the codegen output to `color:red;font-weight:bold`.
fn interleave_semicolons(values: Vec<ComponentValue>) -> Vec<ComponentValue> {
  let mut result: Vec<ComponentValue> = Vec::with_capacity(values.len() * 2);
  let mut prev_was_decl = false;
  for cv in values {
    let is_decl = matches!(cv, ComponentValue::Declaration(_));
    if is_decl && prev_was_decl {
      result.push(ComponentValue::Delimiter(Box::new(Delimiter {
        span: DUMMY_SP,
        value: DelimiterValue::Semicolon,
      })));
    }
    prev_was_decl = is_decl;
    result.push(cv);
  }
  result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Plugin-level tests for the **PostCSS** `non_atomicify_rules_plugin`
/// (the real production path used by `transform_css_via_postcss`).
///
/// These tests drive the production pipeline that `cssMapScoped` actually
/// uses at runtime, catching issues earlier than the `babel-plugin.rs`
/// integration tests or the e2e tests.
///
/// Uses `insta::assert_snapshot!` for inline snapshot assertions. To update
/// snapshots when changes are intentional, run with `INSTA_UPDATE=always` or
/// use `cargo insta review`.
#[cfg(all(test, feature = "postcss_engine"))]
mod postcss_plugin_tests {
  use crate::postcss::transform::{TransformCssOptions, transform_css};

  fn transform(css: &str, class_name: &str) -> Vec<String> {
    let opts = TransformCssOptions {
      atomic: Some(false),
      non_atomic_class_name: Some(class_name.to_string()),
      ..Default::default()
    };
    transform_css(css, opts)
      .expect("transform succeeded")
      .sheets
  }

  /// Convenience: run the transform with a fixed dummy class name and join
  /// all sheets with newlines for compact snapshot diffs.
  ///
  /// Snapshot tests use `insta::assert_snapshot!(snapshot(css), @"...")` to
  /// lock down the exact transformed output. To intentionally update a
  /// snapshot when changes are expected, run with `INSTA_UPDATE=always` or
  /// use `cargo insta review` after running the test suite.
  fn snapshot(css: &str) -> String {
    transform(css, "cc-xxxxxx").join("\n")
  }

  #[test]
  fn user_select_gets_vendor_prefixes() {
    // Contract: autoprefixer must produce -webkit / -moz prefixes for
    // user-select, with the unprefixed property following.
    let out = snapshot(".foo{user-select:none}");
    assert!(
      out.contains("-webkit-user-select:none"),
      "missing -webkit prefix: {}",
      out
    );
    assert!(
      out.contains("-moz-user-select:none"),
      "missing -moz prefix: {}",
      out
    );
    insta::assert_snapshot!(out, @".cc-xxxxxx .foo{-webkit-user-select:none;-moz-user-select:none;user-select:none}");
  }

  #[test]
  fn multiple_decls_remain_grouped_under_one_class() {
    // Contract: all decls of a variant share ONE class (defining property
    // of non-atomic mode). Snapshot locks the exact grouped output.
    let out = snapshot(".foo{color:red;font-weight:bold;margin-top:8px}");
    assert_eq!(
      out.matches(".cc-xxxxxx").count(),
      1,
      "must be exactly one class selector: {}",
      out
    );
    insta::assert_snapshot!(out, @".cc-xxxxxx .foo{color:red;font-weight:bold;margin-top:8px}");
  }

  #[test]
  fn nested_at_rule_bare_decls_emit_correctly() {
    // Regression test: `@media` with bare decls inside a parent rule was
    // being dropped before the at_rule_filter_exit bare-decl handling was
    // added. Snapshot locks the exact 2-sheet output (one for the parent
    // rule's direct decl, one for the @media-wrapped inner decl).
    //
    // Note: optimiser converts `12px` to `9pt` (equivalent literal).
    let out = snapshot(".editor .panel{display:grid;@media (min-width:1px){row-gap:12px}}");
    assert!(
      out.contains("@media"),
      "@media wrapper must be preserved: {}",
      out
    );
    insta::assert_snapshot!(out, @r"
    .cc-xxxxxx .editor .panel{display:grid}
    @media (min-width:1px){.cc-xxxxxx .editor .panel{row-gap:9pt}}
    ");
  }

  #[test]
  fn nested_supports_containing_media_preserves_both() {
    let out = snapshot(
      "@supports (display:grid){.editor .panel{display:grid;@media (min-width:1px){row-gap:12px}}}",
    );
    assert!(
      out.contains("@supports") && out.contains("@media"),
      "both wrappers must be preserved: {}",
      out
    );
    insta::assert_snapshot!(out, @r"
    @supports (display:grid){@media (min-width:1px){.cc-xxxxxx .editor .panel{row-gap:9pt}}}
    @supports (display:grid){.cc-xxxxxx .editor .panel{display:grid}}
    ");
  }

  #[test]
  fn parent_pseudo_hover_is_flattened_against_parent() {
    // `&:hover` inside parent rule must flatten to `.parent:hover` then be
    // scoped under .cc-xxxxxx.
    let out = snapshot(".editor .panel{&:hover{cursor:pointer}}");
    insta::assert_snapshot!(out, @".cc-xxxxxx .editor .panel:hover{cursor:pointer}");
  }

  #[test]
  fn keyframes_step_keywords_are_not_scoped() {
    // Critical contract: `from`/`to`/`%` step keywords inside `@keyframes`
    // must NOT be prefixed with .cc-xxxxxx (they're step keywords, not
    // selectors). Note: optimiser rewrites `from` to `0%`.
    let out = snapshot("@keyframes fadeIn{from{opacity:0}to{opacity:1}}");
    assert!(
      !out.contains(".cc-xxxxxx 0%"),
      "step keyword must not be scoped: {}",
      out
    );
    assert!(
      !out.contains(".cc-xxxxxx from"),
      "step keyword must not be scoped: {}",
      out
    );
    assert!(
      !out.contains(".cc-xxxxxx to"),
      "step keyword must not be scoped: {}",
      out
    );
    insta::assert_snapshot!(out, @"@keyframes fadeIn{0%{opacity:0}to{opacity:1}}");
  }

  #[test]
  fn comma_separated_selectors_each_get_scoped() {
    let out = snapshot(".a, .b{color:red}");
    insta::assert_snapshot!(out, @".cc-xxxxxx .a,.cc-xxxxxx .b{color:red}");
  }

  #[test]
  fn comma_separated_descendant_selectors_each_scope_independently() {
    // Each selector in the comma list scoped independently; decl appears once
    // (shared between both targets in the comma list).
    let out = snapshot(".editor .panel, .editor .panel-title{opacity:0.95}");
    assert_eq!(
      out.matches("opacity").count(),
      1,
      "decl must appear exactly once: {}",
      out
    );
    insta::assert_snapshot!(out, @".cc-xxxxxx .editor .panel,.cc-xxxxxx .editor .panel-title{opacity:.95}");
  }

  #[test]
  fn parent_ampersand_with_descendant_child_selector_flattens_correctly() {
    // Editor pattern: `.editor .panel`: { `& .panel-icon`: {...} }
    // `&` refers to the outer descendant chain `.editor .panel`, producing
    // `.cc-xxxxxx .editor .panel .panel-icon`.
    let out = snapshot(".editor .panel{& .panel-icon{color:red}}");
    insta::assert_snapshot!(out, @".cc-xxxxxx .editor .panel .panel-icon{color:red}");
  }

  #[test]
  fn right_ampersand_with_attribute_prefix_for_rtl() {
    // Editor i18n pattern: `[dir="rtl"] &`: { `.editor blockquote`: {...} }
    // `&` on the right side of `[dir="rtl"] &` means the variant class
    // attaches at the END, with `[dir="rtl"]` as ancestor.
    let out = snapshot("[dir=\"rtl\"] &{.editor blockquote{padding-left:0}}");
    insta::assert_snapshot!(out, @"[dir=rtl] .cc-xxxxxx .editor blockquote{padding-left:0}");
  }

  #[test]
  fn empty_input_produces_no_sheets() {
    let out = snapshot("");
    insta::assert_snapshot!(out, @"");
  }
}
