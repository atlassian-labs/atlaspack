use std::cmp::Ordering;

use swc_core::css::ast::{ComponentValue, Declaration, DeclarationName, Rule, Stylesheet};

use super::super::transform::{Plugin, TransformContext};

#[derive(Debug, Default, Clone, Copy)]
pub struct SortShorthandDeclarations;

impl Plugin for SortShorthandDeclarations {
  fn name(&self) -> &'static str {
    "sort-shorthand-declarations"
  }

  fn run(&self, stylesheet: &mut Stylesheet, _ctx: &mut TransformContext<'_>) {
    sort_stylesheet(stylesheet);
  }
}

pub fn sort_shorthand_declarations() -> SortShorthandDeclarations {
  SortShorthandDeclarations
}

fn sort_stylesheet(stylesheet: &mut Stylesheet) {
  sort_rules(&mut stylesheet.rules);
}

pub(crate) fn sort_rules(rules: &mut Vec<Rule>) {
  for rule in &mut *rules {
    match rule {
      Rule::QualifiedRule(rule) => sort_component_values(&mut rule.block.value),
      Rule::AtRule(at_rule) => {
        if let Some(block) = &mut at_rule.block {
          sort_component_values(&mut block.value);
        }
      }
      Rule::ListOfComponentValues(list) => sort_component_values(&mut list.children),
    }
  }

  rules.sort_by(|a, b| {
    compare_declaration_buckets(first_declaration_in_rule(a), first_declaration_in_rule(b))
  });

  if std::env::var("COMPILED_CSS_TRACE").is_ok() {
    let summary: Vec<String> = rules
      .iter()
      .map(|r| {
        first_declaration_in_rule(r)
          .map(|d| match &d.name {
            DeclarationName::Ident(i) => i.value.to_string(),
            DeclarationName::DashedIdent(i) => i.value.to_string(),
          })
          .unwrap_or_else(|| "<none>".to_string())
      })
      .collect();
    eprintln!("[sort-shorthand] order=[{}]", summary.join(", "));
  }
}

fn sort_component_values(values: &mut Vec<ComponentValue>) {
  for value in &mut *values {
    match value {
      ComponentValue::QualifiedRule(rule) => sort_component_values(&mut rule.block.value),
      ComponentValue::AtRule(at_rule) => {
        if let Some(block) = &mut at_rule.block {
          sort_component_values(&mut block.value);
        }
      }
      ComponentValue::SimpleBlock(block) => sort_component_values(&mut block.value),
      ComponentValue::ListOfComponentValues(list) => sort_component_values(&mut list.children),
      ComponentValue::KeyframeBlock(block) => sort_component_values(&mut block.block.value),
      _ => {}
    }
  }

  values.sort_by(|a, b| {
    compare_declaration_buckets(
      first_declaration_in_component(a),
      first_declaration_in_component(b),
    )
  });
}

fn compare_declaration_buckets(a: Option<&Declaration>, b: Option<&Declaration>) -> Ordering {
  match (a, b) {
    (Some(a_decl), Some(b_decl)) => {
      let a_bucket = shorthand_bucket_for_declaration(a_decl).unwrap_or(u32::MAX);
      let b_bucket = shorthand_bucket_for_declaration(b_decl).unwrap_or(u32::MAX);
      a_bucket.cmp(&b_bucket)
    }
    _ => Ordering::Equal,
  }
}

fn shorthand_bucket_for_declaration(declaration: &Declaration) -> Option<u32> {
  let name = match &declaration.name {
    DeclarationName::Ident(ident) => ident.value.as_ref(),
    DeclarationName::DashedIdent(ident) => ident.value.as_ref(),
  };

  // First attempt: direct shorthand bucket match
  if let Some(bucket) = shorthand_bucket(name) {
    return Some(bucket);
  }

  // COMPAT: Babel ordering effectively groups certain longhands with their
  // shorthand for sort priority. Mirror this by mapping known longhands
  // to the bucket of their parent shorthand. This preserves encounter
  // order among equal-bucket items via stable sort, aligning with Babel.
  if let Some(parent) = parent_shorthand_for(name) {
    return shorthand_bucket(parent);
  }

  None
}

pub fn parent_shorthand_for(property: &str) -> Option<&'static str> {
  // Reverse mapping auto-derived from packages/utils/src/shorthand.ts (shorthandFor),
  // choosing the parent shorthand with the minimal bucket depth when multiple apply.
  match property {
    // animation-range
    "animation-range-end" => Some("animation-range"),
    "animation-range-start" => Some("animation-range"),

    // border family
    "border-block-end" => Some("border-block"),
    "border-block-end-color" => Some("border-color"),
    "border-block-end-style" => Some("border-style"),
    "border-block-end-width" => Some("border-width"),
    "border-block-start" => Some("border-block"),
    "border-block-start-color" => Some("border-color"),
    "border-block-start-style" => Some("border-style"),
    "border-block-start-width" => Some("border-width"),
    "border-bottom-color" => Some("border-color"),
    "border-bottom-style" => Some("border-style"),
    "border-bottom-width" => Some("border-width"),
    "border-top-color" => Some("border-color"),
    "border-top-style" => Some("border-style"),
    "border-top-width" => Some("border-width"),
    "border-block-color" => Some("border-color"),
    "border-inline-color" => Some("border-color"),
    "border-inline-start-color" => Some("border-color"),
    "border-inline-end-color" => Some("border-color"),
    "border-left-color" => Some("border-color"),
    "border-right-color" => Some("border-color"),
    "border-image-outset" => Some("border-image"),
    "border-image-repeat" => Some("border-image"),
    "border-image-slice" => Some("border-image"),
    "border-image-source" => Some("border-image"),
    "border-image-width" => Some("border-image"),
    "border-inline-end" => Some("border-inline"),
    "border-inline-end-style" => Some("border-style"),
    "border-inline-end-width" => Some("border-width"),
    "border-inline-start" => Some("border-inline"),
    "border-inline-start-style" => Some("border-style"),
    "border-inline-start-width" => Some("border-width"),
    "border-left-style" => Some("border-style"),
    "border-left-width" => Some("border-width"),
    "border-right-style" => Some("border-style"),
    "border-right-width" => Some("border-width"),
    "border-bottom-left-radius" => Some("border-radius"),
    "border-bottom-right-radius" => Some("border-radius"),
    "border-end-end-radius" => Some("border-radius"),
    "border-end-start-radius" => Some("border-radius"),
    "border-start-end-radius" => Some("border-radius"),
    "border-start-start-radius" => Some("border-radius"),
    "border-top-left-radius" => Some("border-radius"),
    "border-top-right-radius" => Some("border-radius"),
    "border-block-style" => Some("border-style"),
    "border-inline-style" => Some("border-style"),
    "border-block-width" => Some("border-width"),
    "border-inline-width" => Some("border-width"),

    // column-rule
    "column-rule-color" => Some("column-rule"),
    "column-rule-style" => Some("column-rule"),
    "column-rule-width" => Some("column-rule"),

    // contain-intrinsic-size
    "contain-intrinsic-block-size" => Some("contain-intrinsic-size"),
    "contain-intrinsic-height" => Some("contain-intrinsic-size"),
    "contain-intrinsic-inline-size" => Some("contain-intrinsic-size"),
    "contain-intrinsic-width" => Some("contain-intrinsic-size"),

    // flex-flow
    "flex-direction" => Some("flex-flow"),
    "flex-wrap" => Some("flex-flow"),

    // font-synthesis
    "font-synthesis-position" => Some("font-synthesis"),
    "font-synthesis-small-caps" => Some("font-synthesis"),
    "font-synthesis-style" => Some("font-synthesis"),
    "font-synthesis-weight" => Some("font-synthesis"),

    // font-variant
    "font-variant-alternates" => Some("font-variant"),
    "font-variant-caps" => Some("font-variant"),
    "font-variant-east-asian" => Some("font-variant"),
    "font-variant-emoji" => Some("font-variant"),
    "font-variant-ligatures" => Some("font-variant"),
    "font-variant-numeric" => Some("font-variant"),
    "font-variant-position" => Some("font-variant"),

    // grid
    "grid-column" => Some("grid-area"),
    "grid-column-end" => Some("grid-area"),
    "grid-column-start" => Some("grid-area"),
    "grid-row" => Some("grid-area"),
    "grid-row-end" => Some("grid-area"),
    "grid-row-start" => Some("grid-area"),
    "grid-template-rows" => Some("grid-template"),
    "grid-template-columns" => Some("grid-template"),
    "grid-template-areas" => Some("grid-template"),

    // inset-block/inline
    "inset-block-start" => Some("inset-block"),
    "inset-block-end" => Some("inset-block"),
    "top" => Some("inset-block"),
    "bottom" => Some("inset-block"),
    "inset-inline-start" => Some("inset-inline"),
    "inset-inline-end" => Some("inset-inline"),
    "left" => Some("inset-inline"),
    "right" => Some("inset-inline"),

    // list-style
    "list-style-image" => Some("list-style"),
    "list-style-position" => Some("list-style"),
    "list-style-type" => Some("list-style"),

    // margin block/inline
    "margin-block-start" => Some("margin-block"),
    "margin-block-end" => Some("margin-block"),
    "margin-top" => Some("margin"),
    "margin-bottom" => Some("margin"),
    "margin-inline-start" => Some("margin-inline"),
    "margin-inline-end" => Some("margin-inline"),
    "margin-left" => Some("margin"),
    "margin-right" => Some("margin"),

    // mask-border
    "mask-border-mode" => Some("mask-border"),
    "mask-border-outset" => Some("mask-border"),
    "mask-border-repeat" => Some("mask-border"),
    "mask-border-slice" => Some("mask-border"),
    "mask-border-source" => Some("mask-border"),
    "mask-border-width" => Some("mask-border"),

    // overscroll-behavior
    "overscroll-behavior-x" => Some("overscroll-behavior"),
    "overscroll-behavior-y" => Some("overscroll-behavior"),
    "overscroll-behavior-inline" => Some("overscroll-behavior"),
    "overscroll-behavior-block" => Some("overscroll-behavior"),

    // padding block/inline
    "padding-block-start" => Some("padding-block"),
    "padding-block-end" => Some("padding-block"),
    "padding-top" => Some("padding"),
    "padding-bottom" => Some("padding"),
    "padding-inline-start" => Some("padding-inline"),
    "padding-inline-end" => Some("padding-inline"),
    "padding-left" => Some("padding"),
    "padding-right" => Some("padding"),

    // place-*
    "align-content" => Some("place-content"),
    "justify-content" => Some("place-content"),
    "align-items" => Some("place-items"),
    "justify-items" => Some("place-items"),
    "align-self" => Some("place-self"),
    "justify-self" => Some("place-self"),

    // position-try
    "position-try-order" => Some("position-try"),
    "position-try-fallbacks" => Some("position-try"),

    // scroll-margin
    "scroll-margin-block" => Some("scroll-margin"),
    "scroll-margin-block-end" => Some("scroll-margin"),
    "scroll-margin-block-start" => Some("scroll-margin"),
    "scroll-margin-bottom" => Some("scroll-margin"),
    "scroll-margin-inline" => Some("scroll-margin"),
    "scroll-margin-inline-end" => Some("scroll-margin"),
    "scroll-margin-inline-start" => Some("scroll-margin"),
    "scroll-margin-left" => Some("scroll-margin"),
    "scroll-margin-right" => Some("scroll-margin"),
    "scroll-margin-top" => Some("scroll-margin"),

    // scroll-padding
    "scroll-padding-block" => Some("scroll-padding"),
    "scroll-padding-block-end" => Some("scroll-padding"),
    "scroll-padding-block-start" => Some("scroll-padding"),
    "scroll-padding-bottom" => Some("scroll-padding"),
    "scroll-padding-inline" => Some("scroll-padding"),
    "scroll-padding-inline-end" => Some("scroll-padding"),
    "scroll-padding-inline-start" => Some("scroll-padding"),
    "scroll-padding-left" => Some("scroll-padding"),
    "scroll-padding-right" => Some("scroll-padding"),
    "scroll-padding-top" => Some("scroll-padding"),

    // scroll-timeline
    "scroll-timeline-name" => Some("scroll-timeline"),
    "scroll-timeline-axis" => Some("scroll-timeline"),

    // text-* groups
    "text-decoration-color" => Some("text-decoration"),
    "text-decoration-line" => Some("text-decoration"),
    "text-decoration-style" => Some("text-decoration"),
    "text-decoration-thickness" => Some("text-decoration"),
    "text-emphasis-color" => Some("text-emphasis"),
    "text-emphasis-style" => Some("text-emphasis"),
    "text-wrap-mode" => Some("text-wrap"),
    "text-wrap-style" => Some("text-wrap"),

    // view-timeline
    "view-timeline-name" => Some("view-timeline"),
    "view-timeline-axis" => Some("view-timeline"),

    _ => None,
  }
}

fn first_declaration_in_rule(rule: &Rule) -> Option<&Declaration> {
  match rule {
    Rule::QualifiedRule(rule) => find_first_declaration(&rule.block.value),
    Rule::AtRule(at_rule) => at_rule
      .block
      .as_ref()
      .and_then(|block| find_first_declaration(&block.value)),
    Rule::ListOfComponentValues(list) => find_first_declaration(&list.children),
  }
}

fn first_declaration_in_component(component: &ComponentValue) -> Option<&Declaration> {
  match component {
    ComponentValue::Declaration(declaration) => Some(declaration),
    ComponentValue::QualifiedRule(rule) => find_first_declaration(&rule.block.value),
    ComponentValue::AtRule(at_rule) => at_rule
      .block
      .as_ref()
      .and_then(|block| find_first_declaration(&block.value)),
    ComponentValue::SimpleBlock(block) => find_first_declaration(&block.value),
    ComponentValue::ListOfComponentValues(list) => find_first_declaration(&list.children),
    ComponentValue::KeyframeBlock(block) => find_first_declaration(&block.block.value),
    _ => None,
  }
}

fn find_first_declaration(values: &[ComponentValue]) -> Option<&Declaration> {
  for value in values {
    if let Some(declaration) = first_declaration_in_component(value) {
      return Some(declaration);
    }
  }

  None
}

pub fn shorthand_bucket(property: &str) -> Option<u32> {
  let bucket = match property {
    "all" => 0,
    "animation"
    | "animation-range"
    | "background"
    | "border"
    | "border-image"
    | "border-radius"
    | "column-rule"
    | "columns"
    | "contain-intrinsic-size"
    | "container"
    | "flex"
    | "flex-flow"
    | "font"
    | "font-synthesis"
    | "gap"
    | "grid"
    | "grid-area"
    | "inset"
    | "list-style"
    | "margin"
    | "mask"
    | "mask-border"
    | "offset"
    | "outline"
    | "overflow"
    | "overscroll-behavior"
    | "padding"
    | "place-content"
    | "place-items"
    | "place-self"
    | "position-try"
    | "scroll-margin"
    | "scroll-padding"
    | "scroll-timeline"
    | "text-decoration"
    | "text-emphasis"
    | "text-wrap"
    | "transition"
    | "view-timeline" => 1,
    "border-color"
    | "border-style"
    | "border-width"
    | "font-variant"
    | "grid-column"
    | "grid-row"
    | "grid-template"
    | "inset-block"
    | "inset-inline"
    | "margin-block"
    | "margin-inline"
    | "padding-block"
    | "padding-inline"
    | "scroll-margin-block"
    | "scroll-margin-inline"
    | "scroll-padding-block"
    | "scroll-padding-inline" => 2,
    "border-block" | "border-inline" => 3,
    "border-top" | "border-right" | "border-bottom" | "border-left" => 4,
    "border-block-start" | "border-block-end" | "border-inline-start" | "border-inline-end" => 5,
    _ => return None,
  };

  Some(bucket)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::postcss::transform::{TransformContext, TransformCssOptions};
  use swc_core::common::{input::StringInput, FileName, SourceMap};
  use swc_core::css::ast::Rule as CssRule;
  use swc_core::css::codegen::{writer::basic::BasicCssWriter, CodeGenerator, CodegenConfig, Emit};
  use swc_core::css::parser::{parse_string_input, parser::ParserConfig};

  fn parse_stylesheet(css: &str) -> Stylesheet {
    let cm: std::sync::Arc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("test.css".into()).into(), css.into());
    let mut errors = vec![];
    parse_string_input::<Stylesheet>(
      StringInput::from(&*fm),
      None,
      ParserConfig::default(),
      &mut errors,
    )
    .expect("failed to parse stylesheet")
  }

  fn declaration_names_from_rule(rule: &CssRule) -> Vec<String> {
    match rule {
      CssRule::QualifiedRule(rule) => collect_declaration_names(&rule.block.value),
      CssRule::AtRule(at_rule) => at_rule
        .block
        .as_ref()
        .map(|block| collect_declaration_names(&block.value))
        .unwrap_or_default(),
      CssRule::ListOfComponentValues(list) => collect_declaration_names(&list.children),
    }
  }

  fn declaration_name_to_string(name: &DeclarationName) -> String {
    match name {
      DeclarationName::Ident(ident) => ident.value.to_string(),
      DeclarationName::DashedIdent(ident) => ident.value.to_string(),
    }
  }

  fn collect_declaration_names(values: &[ComponentValue]) -> Vec<String> {
    let mut names = Vec::new();

    for value in values {
      match value {
        ComponentValue::Declaration(declaration) => {
          names.push(declaration_name_to_string(&declaration.name));
        }
        ComponentValue::QualifiedRule(rule) => {
          names.extend(collect_declaration_names(&rule.block.value));
        }
        ComponentValue::AtRule(at_rule) => {
          if let Some(block) = &at_rule.block {
            names.extend(collect_declaration_names(&block.value));
          }
        }
        ComponentValue::SimpleBlock(block) => {
          names.extend(collect_declaration_names(&block.value));
        }
        ComponentValue::ListOfComponentValues(list) => {
          names.extend(collect_declaration_names(&list.children));
        }
        ComponentValue::KeyframeBlock(block) => {
          names.extend(collect_declaration_names(&block.block.value));
        }
        _ => {}
      }
    }

    names
  }

  fn serialize_stylesheet(stylesheet: &Stylesheet) -> String {
    let mut output = String::new();
    let writer = BasicCssWriter::new(&mut output, None, Default::default());
    let mut generator = CodeGenerator::new(writer, CodegenConfig { minify: false });
    generator
      .emit(stylesheet)
      .expect("failed to serialize stylesheet");
    output
  }

  #[test]
  fn places_shorthand_before_longhand() {
    let mut stylesheet = parse_stylesheet(".a { margin-top: 1px; margin: 0; }");
    let options = TransformCssOptions::default();
    let mut ctx = TransformContext::new(&options);
    SortShorthandDeclarations.run(&mut stylesheet, &mut ctx);

    let rule = stylesheet
      .rules
      .first()
      .expect("expected a rule after sorting");
    let names = declaration_names_from_rule(rule);
    assert_eq!(names, vec!["margin", "margin-top"]);
  }

  #[test]
  fn sorts_nested_blocks() {
    let mut stylesheet =
      parse_stylesheet("@media screen { .a { padding-left: 4px; padding: 0; } }");
    let options = TransformCssOptions::default();
    let mut ctx = TransformContext::new(&options);
    SortShorthandDeclarations.run(&mut stylesheet, &mut ctx);

    let names = stylesheet
      .rules
      .first()
      .and_then(|rule| match rule {
        CssRule::AtRule(at_rule) => {
          at_rule
            .block
            .as_ref()
            .and_then(|block| match block.value.first() {
              Some(ComponentValue::QualifiedRule(rule)) => {
                Some(collect_declaration_names(&rule.block.value))
              }
              _ => None,
            })
        }
        _ => None,
      })
      .expect("expected declarations inside nested block");

    assert_eq!(names, vec!["padding", "padding-left"]);
  }

  #[test]
  fn preserves_nodes_without_declarations() {
    let mut stylesheet =
      parse_stylesheet(".a { /* comment */ color: red; } .b { display: block; }");
    let original = serialize_stylesheet(&stylesheet);
    let options = TransformCssOptions::default();
    let mut ctx = TransformContext::new(&options);
    SortShorthandDeclarations.run(&mut stylesheet, &mut ctx);
    let sorted = serialize_stylesheet(&stylesheet);

    assert_eq!(original, sorted);
  }
}
