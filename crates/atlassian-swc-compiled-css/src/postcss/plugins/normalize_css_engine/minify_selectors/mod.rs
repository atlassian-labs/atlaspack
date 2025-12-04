use once_cell::sync::Lazy;
use postcss as pc;
use regex::Regex;
use swc_core::atoms::Atom;
use swc_core::common::{FileName, SourceMap, input::StringInput};
use swc_core::css::ast::*;
use swc_core::css::parser::{parse_string_input, parser::ParserConfig};

use crate::postcss::utils::selector_stringifier::{
  serialize_complex_selector, serialize_relative_selector, serialize_relative_selector_list,
  serialize_selector_list,
};

// Port of postcss-minify-selectors@5.2.1 using SWC's CSS AST to mirror behaviour.

static PSEUDO_REPLACEMENTS: &[(&str, &str)] = &[
  ("nth-child", "first-child"),
  ("nth-of-type", "first-of-type"),
  ("nth-last-child", "last-child"),
  ("nth-last-of-type", "last-of-type"),
];
static PSEUDO_ELEMENTS: &[&str] = &["before", "after", "first-letter", "first-line"];
static TAG_REPLACEMENTS: &[(&str, &str)] = &[("from", "0%"), ("100%", "to")];
static ESCAPES_HEX_REGEX: Lazy<Regex> =
  Lazy::new(|| Regex::new(r"\\([0-9A-Fa-f]{1,6})[ \t\n\f\r]?").unwrap());
static ESCAPE_ANY_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\\.").unwrap());
static DISALLOWED_RANGE_REGEX: Lazy<Regex> = Lazy::new(|| {
  Regex::new(r"[\u0000-\u002c\u002e\u002f\u003A-\u0040\u005B-\u005E\u0060\u007B-\u009f]").unwrap()
});
static LEADING_NUMBER_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(?:-?\d|--)").unwrap());

fn can_unquote(value: &str) -> bool {
  if value == "-" || value.is_empty() {
    return false;
  }
  let mut v = ESCAPES_HEX_REGEX.replace_all(value, "a").to_string();
  v = ESCAPE_ANY_REGEX.replace_all(&v, "a").to_string();
  !(DISALLOWED_RANGE_REGEX.is_match(&v) || LEADING_NUMBER_REGEX.is_match(&v))
}

fn normalize_attribute_string(str_value: &mut Str) {
  let mut value = str_value.value.to_string();
  value = value
    .replace("\\\n", "")
    .replace("\\r", "")
    .replace("\\f", "");
  str_value.value = Atom::from(value.trim().to_string());
  if let Some(raw) = &str_value.raw {
    let raw_value = raw
      .to_string()
      .replace("\\\n", "")
      .replace("\\r", "")
      .replace("\\f", "");
    str_value.raw = Some(Atom::from(raw_value.trim().to_string()));
  }
}

fn process_attribute_selector(selector: &mut AttributeSelector) {
  selector.name.value.value = Atom::from(selector.name.value.value.to_string().trim());
  selector.name.value.raw = None;
  if let Some(AttributeSelectorValue::Str(value)) = selector.value.as_mut() {
    normalize_attribute_string(value);
    let candidate = value.value.to_string();
    if can_unquote(&candidate) {
      let ident = Ident {
        span: value.span,
        value: Atom::from(candidate),
        raw: None,
      };
      selector.value = Some(AttributeSelectorValue::Ident(ident));
    }
  }
}

fn convert_pseudo_element_to_class(pseudo: &PseudoElementSelector) -> Option<PseudoClassSelector> {
  let ident = pseudo.name.value.to_string().to_lowercase();
  if PSEUDO_ELEMENTS.contains(&ident.as_str()) {
    return Some(PseudoClassSelector {
      span: pseudo.span,
      name: pseudo.name.clone(),
      children: None,
    });
  }
  None
}

fn handle_nth_replacements(selector: &mut PseudoClassSelector, lower: &str) -> bool {
  if let Some(children) = &mut selector.children {
    if matches!(
      lower,
      "nth-child" | "nth-of-type" | "nth-last-child" | "nth-last-of-type"
    ) {
      if let Some(first) = children.first_mut() {
        if let PseudoClassSelectorChildren::AnPlusB(an_plus_b) = first {
          match an_plus_b {
            // even -> 2n
            AnPlusB::Ident(ident) => {
              if ident.value.eq_ignore_ascii_case("even") {
                ident.value = Atom::from("2n");
                ident.raw = None;
              }
            }
            AnPlusB::AnPlusBNotation(notation) => {
              let a = notation.a.unwrap_or(0);
              let b = notation.b.unwrap_or(0);

              // a simple one case: 1 -> first/last
              if a == 0 && b == 1 {
                for (src, dst) in PSEUDO_REPLACEMENTS {
                  if lower == *src {
                    selector.name.value = Atom::from(*dst);
                    selector.name.raw = None;
                    selector.children = None;
                    return true;
                  }
                }
              }

              // 2n+1 -> odd
              if a == 2 && b == 1 {
                let ident = Ident {
                  span: notation.span,
                  value: Atom::from("odd"),
                  raw: None,
                };
                *an_plus_b = AnPlusB::Ident(ident);
                return true;
              }
            }
          }
        }
      }
    }
  }
  false
}

fn process_pseudo_element_selector(selector: &mut PseudoElementSelector) {
  selector.name.raw = None;
}

fn process_pseudo_class_selector(selector: &mut PseudoClassSelector) {
  let name = selector.name.value.to_string();
  let lower = name.to_lowercase();
  if selector.children.is_some() {
    if handle_nth_replacements(selector, &lower) {
      return;
    }
  }
  if let Some(children) = &mut selector.children {
    for child in children.iter_mut() {
      match child {
        PseudoClassSelectorChildren::SelectorList(list) => process_selector_list(list, false),
        PseudoClassSelectorChildren::RelativeSelectorList(list) => {
          process_relative_selector_list(list, false)
        }
        PseudoClassSelectorChildren::ForgivingSelectorList(list) => {
          for selector in &mut list.children {
            if let ForgivingComplexSelector::ComplexSelector(sel) = selector {
              process_complex_selector(sel);
            }
          }
          dedupe_forgiving_selectors(&mut list.children);
        }
        PseudoClassSelectorChildren::ForgivingRelativeSelectorList(list) => {
          for selector in &mut list.children {
            if let ForgivingRelativeSelector::RelativeSelector(rel) = selector {
              if let Some(comb) = &mut rel.combinator {
                process_combinator(comb);
              }
              process_complex_selector(&mut rel.selector);
            }
          }
          dedupe_forgiving_relative_selectors(&mut list.children);
        }
        _ => {}
      }
    }
  }
}

fn process_tag_selector(tag: &mut TagNameSelector, is_simple: bool) {
  if !is_simple {
    return;
  }
  let lower = tag.name.value.value.to_string().to_lowercase();
  for (src, rep) in TAG_REPLACEMENTS {
    if lower == *src {
      tag.name.value.value = Atom::from(*rep);
      tag.name.value.raw = None;
      return;
    }
  }
}

fn should_remove_universal(compound: &CompoundSelector) -> bool {
  if let Some(next) = compound.subclass_selectors.first() {
    match next {
      _ => return true,
    }
  }
  false
}

fn process_subclass_selector(subclass: &mut SubclassSelector) {
  match subclass {
    SubclassSelector::Attribute(a) => process_attribute_selector(a),
    SubclassSelector::PseudoClass(p) => process_pseudo_class_selector(p),
    SubclassSelector::PseudoElement(p) => {
      if let Some(mut converted) = convert_pseudo_element_to_class(p) {
        process_pseudo_class_selector(&mut converted);
        *subclass = SubclassSelector::PseudoClass(converted);
      } else {
        process_pseudo_element_selector(p);
      }
    }
    _ => {}
  }
}

fn process_combinator(_c: &mut Combinator) {}

fn process_compound_selector(compound: &mut CompoundSelector, is_simple: bool) {
  if let Some(ty) = &mut compound.type_selector {
    match &mut **ty {
      TypeSelector::TagName(tag) => process_tag_selector(tag, is_simple),
      TypeSelector::Universal(_) => {
        if should_remove_universal(compound) {
          compound.type_selector = None;
        }
      }
    }
  }
  for subclass in &mut compound.subclass_selectors {
    process_subclass_selector(subclass);
  }
}

fn process_complex_selector(selector: &mut ComplexSelector) {
  let child_count = selector.children.len();
  for child in &mut selector.children {
    match child {
      ComplexSelectorChildren::CompoundSelector(comp) => {
        let is_simple = child_count == 1;
        process_compound_selector(comp, is_simple);
      }
      ComplexSelectorChildren::Combinator(c) => {
        process_combinator(c);
      }
    }
  }
}

fn sort_complex_selectors(selectors: &mut Vec<ComplexSelector>) {
  selectors.sort_by(|a, b| format_complex(a).cmp(&format_complex(b)));
}
fn sort_relative_selectors(selectors: &mut Vec<RelativeSelector>) {
  selectors.sort_by(|a, b| format_relative(a).cmp(&format_relative(b)));
}

fn dedupe_complex_selectors(selectors: &mut Vec<ComplexSelector>) {
  let mut seen = std::collections::HashSet::new();
  selectors.retain(|s| seen.insert(format_complex(s)));
}
fn dedupe_relative_selectors(selectors: &mut Vec<RelativeSelector>) {
  let mut seen = std::collections::HashSet::new();
  selectors.retain(|s| seen.insert(format_relative(s)));
}
fn dedupe_forgiving_selectors(selectors: &mut Vec<ForgivingComplexSelector>) {
  let mut seen = std::collections::HashSet::new();
  selectors.retain(|s| {
    if let ForgivingComplexSelector::ComplexSelector(sel) = s {
      seen.insert(format_complex(sel))
    } else {
      true
    }
  });
}
fn dedupe_forgiving_relative_selectors(selectors: &mut Vec<ForgivingRelativeSelector>) {
  let mut seen = std::collections::HashSet::new();
  selectors.retain(|s| {
    if let ForgivingRelativeSelector::RelativeSelector(sel) = s {
      seen.insert(format_relative(sel))
    } else {
      true
    }
  });
}

fn format_complex(sel: &ComplexSelector) -> String {
  serialize_complex_selector(sel)
}
fn format_relative(sel: &RelativeSelector) -> String {
  serialize_relative_selector(sel)
}

fn process_selector_list(list: &mut SelectorList, allow_reorder: bool) {
  for complex in &mut list.children {
    process_complex_selector(complex);
  }
  dedupe_complex_selectors(&mut list.children);
  if allow_reorder {
    sort_complex_selectors(&mut list.children);
  }
}
fn process_relative_selector_list(list: &mut RelativeSelectorList, allow_reorder: bool) {
  for rel in &mut list.children {
    if let Some(c) = &mut rel.combinator {
      process_combinator(c);
    }
    process_complex_selector(&mut rel.selector);
  }
  dedupe_relative_selectors(&mut list.children);
  if allow_reorder {
    sort_relative_selectors(&mut list.children);
  }
}

fn starts_with_relative_combinator(selector: &str) -> bool {
  let trimmed = selector.trim_start();
  if trimmed.starts_with("||") {
    return true;
  }
  matches!(trimmed.chars().next(), Some('>') | Some('+') | Some('~'))
}

fn minify_selector_string(selector: &str) -> Option<String> {
  let trace = std::env::var("COMPILED_CSS_TRACE").is_ok();
  fn log_nth(list: &SelectorList, label: &str) {
    if std::env::var("COMPILED_CSS_TRACE").is_err() {
      return;
    }
    for complex in &list.children {
      for child in &complex.children {
        if let ComplexSelectorChildren::CompoundSelector(compound) = child {
          for subclass in &compound.subclass_selectors {
            if let SubclassSelector::PseudoClass(pc) = subclass {
              if pc.name.value.eq_ignore_ascii_case("nth-of-type") {
                if let Some(children) = &pc.children {
                  if let Some(PseudoClassSelectorChildren::AnPlusB(AnPlusB::AnPlusBNotation(
                    notation,
                  ))) = children.first()
                  {
                    eprintln!(
                      "[minify-selectors] {} a_raw={:?} a={:?} b_raw={:?} b={:?}",
                      label, notation.a_raw, notation.a, notation.b_raw, notation.b
                    );
                  }
                }
              }
            }
          }
        }
      }
    }
  }

  // COMPAT: cssnano postcss-minify-selectors replaces keyframe step tags
  // literally using postcss-selector-parser, so 'from' -> '0%' and '100%' -> 'to'
  // without going through a selector AST/codegen. Mirroring this avoids SWC
  // escaping (e.g. '0%' -> '\\30 \\%').
  let trimmed = selector.trim();
  let lower = trimmed.to_ascii_lowercase();
  if lower == "from" {
    return Some("0%".to_string());
  }
  if lower == "100%" {
    return Some("to".to_string());
  }
  if trimmed.is_empty() {
    return Some(String::new());
  }
  let parse_relative = starts_with_relative_combinator(trimmed);
  if trace {
    eprintln!(
      "[minify-selectors] input='{}' parse_relative={}",
      trimmed, parse_relative
    );
  }
  let cm: SourceMap = Default::default();
  let fm = cm.new_source_file(
    FileName::Custom("sel.css".into()).into(),
    trimmed.to_string(),
  );
  let mut errors = vec![];
  if parse_relative {
    let mut list = parse_string_input::<RelativeSelectorList>(
      StringInput::from(&*fm),
      None,
      ParserConfig::default(),
      &mut errors,
    )
    .ok()?;
    if !errors.is_empty() {
      if trace {
        eprintln!(
          "[minify-selectors] parse errors relative for '{}': {:?}",
          selector, errors
        );
      }
      return None;
    }
    process_relative_selector_list(&mut list, true);
    log_nth(
      &SelectorList {
        span: Default::default(),
        children: list
          .children
          .iter()
          .map(|rel| ComplexSelector {
            span: rel.span,
            children: rel.selector.children.clone(),
          })
          .collect(),
      },
      "relative-an+b",
    );
    let optimized = serialize_relative_selector_list(&list);
    if std::env::var("COMPILED_CSS_TRACE").is_ok() && selector.contains("nth-of-type") {
      eprintln!(
        "[minify-selectors] relative in='{}' out='{}'",
        selector, optimized
      );
    }
    return Some(optimized);
  }
  let mut list = parse_string_input::<SelectorList>(
    StringInput::from(&*fm),
    None,
    ParserConfig::default(),
    &mut errors,
  )
  .ok()?;
  if !errors.is_empty() {
    if trace {
      eprintln!(
        "[minify-selectors] parse errors selector for '{}': {:?}",
        selector, errors
      );
    }
    return None;
  }
  process_selector_list(&mut list, true);
  log_nth(&list, "an+b");
  let optimized = serialize_selector_list(&list);
  if std::env::var("COMPILED_CSS_TRACE").is_ok() && selector.contains("nth-of-type") {
    eprintln!("[minify-selectors] in='{}' out='{}'", selector, optimized);
  }
  if trace {
    eprintln!(
      "[minify-selectors] optimized '{}' -> '{}'",
      selector, optimized
    );
  }
  Some(optimized)
}

pub fn plugin() -> pc::BuiltPlugin {
  let cache = std::sync::Mutex::new(std::collections::HashMap::<String, String>::new());
  pc::plugin("postcss-minify-selectors")
    .once_exit(move |css, _| {
      match css {
        pc::ast::nodes::RootLike::Root(root) => {
          root.walk_rules(|node, _| {
            if let Some(rule) = postcss::ast::nodes::as_rule(&node) {
              let selector = rule.selector();
              if selector.ends_with(':') {
                return true;
              }
              if let Some(v) = cache.lock().unwrap().get(&selector).cloned() {
                rule.set_selector(v);
                return true;
              }
              if let Some(optimized) = minify_selector_string(&selector) {
                cache
                  .lock()
                  .unwrap()
                  .insert(selector.clone(), optimized.clone());
                rule.set_selector(optimized);
              }
            }
            true
          });
        }
        pc::ast::nodes::RootLike::Document(doc) => {
          doc.walk_rules(|node, _| {
            if let Some(rule) = postcss::ast::nodes::as_rule(&node) {
              let selector = rule.selector();
              if selector.ends_with(':') {
                return true;
              }
              if let Some(v) = cache.lock().unwrap().get(&selector).cloned() {
                rule.set_selector(v);
                return true;
              }
              if let Some(optimized) = minify_selector_string(&selector) {
                cache
                  .lock()
                  .unwrap()
                  .insert(selector.clone(), optimized.clone());
                rule.set_selector(optimized);
              }
            }
            true
          });
        }
      }
      Ok(())
    })
    .build()
}

#[cfg(test)]
mod tests {
  use super::minify_selector_string;

  #[test]
  fn trims_whitespace_inside_is_arguments() {
    let optimized = minify_selector_string(">:is(div, button)").unwrap();
    assert_eq!(optimized, ">:is(div,button)");
  }

  #[test]
  fn keeps_explicit_one_multiplier_in_an_plus_b() {
    let optimized = minify_selector_string("div:nth-of-type(1n+4)").unwrap();
    assert_eq!(optimized, "div:nth-of-type(1n+4)");
  }

  #[test]
  fn trims_whitespace_around_child_combinators() {
    let optimized = minify_selector_string("> div > div:first-of-type").unwrap();
    assert_eq!(optimized, ">div>div:first-of-type");
  }
}
