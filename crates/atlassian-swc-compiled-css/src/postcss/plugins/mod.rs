#![allow(dead_code)]

#[path = "at-rules/mod.rs"]
pub mod at_rules;
#[path = "atomicify-rules.rs"]
pub mod atomicify_rules;
// colormin is not yet enabled; when parity requires it we can add it back.
// #[path = "colormin.rs"]
// pub mod colormin;
#[path = "colormin_lite.rs"]
pub mod colormin_lite;
#[path = "convert-values.rs"]
pub mod convert_values;
#[path = "discard-comments.rs"]
pub mod discard_comments;
#[path = "discard-duplicates.rs"]
pub mod discard_duplicates;
#[path = "discard-empty-rules.rs"]
pub mod discard_empty_rules;
#[path = "expand-shorthands/mod.rs"]
pub mod expand_shorthands;
#[path = "extract-stylesheets.rs"]
pub mod extract_stylesheets;
#[path = "flatten-multiple-selectors.rs"]
pub mod flatten_multiple_selectors;
#[path = "increase-specificity.rs"]
pub mod increase_specificity;
#[path = "merge-duplicate-at-rules.rs"]
pub mod merge_duplicate_at_rules;
#[path = "minify-params.rs"]
pub mod minify_params;
#[path = "minify-selectors.rs"]
pub mod minify_selectors;
#[path = "nested.rs"]
pub mod nested;
#[path = "normalize-css.rs"]
pub mod normalize_css;
#[path = "normalize_css_engine/mod.rs"]
pub mod normalize_css_engine;
#[path = "normalize-current-color.rs"]
pub mod normalize_current_color;
#[path = "normalize-whitespace.rs"]
pub mod normalize_whitespace;
// #[path = "ordered-values.rs"]
// pub mod ordered_values;
#[cfg(feature = "postcss_engine")]
#[path = "expand_shorthands_engine.rs"]
pub mod expand_shorthands_engine;
#[path = "parent-orphaned-pseudos.rs"]
pub mod parent_orphaned_pseudos;
#[path = "reduce-initial/mod.rs"]
pub mod reduce_initial;
#[path = "sort-atomic-style-sheet.rs"]
pub mod sort_atomic_style_sheet;
#[path = "sort-shorthand-declarations.rs"]
pub mod sort_shorthand_declarations;
#[path = "vendor_autoprefixer/mod.rs"]
pub mod vendor_autoprefixer;
#[path = "vendor_prefixing_lite.rs"]
pub mod vendor_prefixing_lite;
#[allow(unused_imports)]
pub use atomicify_rules::*;

use super::transform::{Plugin, TransformContext};
use swc_core::css::ast::Stylesheet;

/// Placeholder representing yet-to-be-ported cssnano plugins.
#[derive(Debug, Clone)]
pub struct CssnanoPlaceholder {
  name: &'static str,
}

impl CssnanoPlaceholder {
  pub fn new(name: &'static str) -> Self {
    Self { name }
  }
}

impl Plugin for CssnanoPlaceholder {
  fn name(&self) -> &'static str {
    self.name
  }

  fn run(&self, _stylesheet: &mut Stylesheet, _ctx: &mut TransformContext<'_>) {}
}
