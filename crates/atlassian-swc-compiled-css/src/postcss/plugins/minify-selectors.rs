use indexmap::IndexSet;
use swc_core::atoms::Atom;
use swc_core::css::ast::{
  AnPlusB, AttributeSelector, AttributeSelectorValue, ComplexSelector, ComplexSelectorChildren,
  ComponentValue, CompoundSelector, Ident, KeyframeSelector, Number, Percentage,
  PseudoClassSelector, PseudoClassSelectorChildren, PseudoElementSelector, QualifiedRule,
  QualifiedRulePrelude, RelativeSelector, RelativeSelectorList, Rule, SelectorList, SimpleBlock,
  Stylesheet, SubclassSelector, TypeSelector,
};

use super::super::transform::{Plugin, TransformContext};
use crate::postcss::utils::selector_stringifier;

static PSEUDO_REPLACEMENTS: &[(&str, &str)] = &[
  ("nth-child", "first-child"),
  ("nth-of-type", "first-of-type"),
  ("nth-last-child", "last-child"),
  ("nth-last-of-type", "last-of-type"),
];

static PSEUDO_ELEMENTS: &[&str] = &["before", "after", "first-letter", "first-line"];
static TAG_REPLACEMENTS: &[(&str, &str)] = &[("from", "0%"), ("100%", "to")];

#[derive(Debug, Clone, Copy)]
pub struct MinifySelectors {
  sort: bool,
}

impl MinifySelectors {
  fn new(sort: bool) -> Self {
    Self { sort }
  }
}

impl Plugin for MinifySelectors {
  fn name(&self) -> &'static str {
    "postcss-minify-selectors"
  }

  fn run(&self, stylesheet: &mut Stylesheet, _ctx: &mut TransformContext<'_>) {
    process_stylesheet(stylesheet, self.sort);
  }
}

pub fn minify_selectors(sort: bool) -> MinifySelectors {
  MinifySelectors::new(sort)
}

fn process_stylesheet(stylesheet: &mut Stylesheet, sort: bool) {
  for rule in &mut stylesheet.rules {
    process_rule(rule, sort);
  }
}

fn process_rule(rule: &mut Rule, sort: bool) {
  match rule {
    Rule::QualifiedRule(rule) => {
      process_qualified_rule(rule, sort);
      process_simple_block(&mut rule.block, sort);
    }
    Rule::AtRule(at_rule) => {
      if let Some(block) = &mut at_rule.block {
        process_simple_block(block, sort);
      }
    }
    Rule::ListOfComponentValues(list) => process_component_values(&mut list.children, sort),
  }
}

fn process_simple_block(block: &mut SimpleBlock, sort: bool) {
  process_component_values(&mut block.value, sort);
}

fn process_component_values(values: &mut [ComponentValue], sort: bool) {
  for value in values {
    match value {
      ComponentValue::QualifiedRule(rule) => {
        process_qualified_rule(rule, sort);
        process_simple_block(&mut rule.block, sort);
      }
      ComponentValue::AtRule(at_rule) => {
        if let Some(block) = &mut at_rule.block {
          process_simple_block(block, sort);
        }
      }
      ComponentValue::SimpleBlock(block) => process_simple_block(block, sort),
      ComponentValue::ListOfComponentValues(list) => {
        process_component_values(&mut list.children, sort)
      }
      ComponentValue::Function(function) => process_component_values(&mut function.value, sort),
      ComponentValue::KeyframeBlock(block) => {
        for selector in &mut block.prelude {
          normalize_keyframe_selector(selector);
        }
        process_simple_block(&mut block.block, sort)
      }
      _ => {}
    }
  }
}

fn normalize_keyframe_selector(selector: &mut KeyframeSelector) {
  match selector {
    KeyframeSelector::Ident(ident) => {
      if ident.value.eq_ignore_ascii_case("from") {
        *selector = KeyframeSelector::Percentage(Percentage {
          span: ident.span,
          value: Number {
            span: ident.span,
            value: 0.0,
            raw: None,
          },
        });
      }
    }
    KeyframeSelector::Percentage(percent) => {
      if (percent.value.value - 100.0).abs() < f64::EPSILON {
        *selector = KeyframeSelector::Ident(Ident {
          span: percent.span,
          value: Atom::from("to"),
          raw: None,
        });
      }
    }
  }
}

fn process_qualified_rule(rule: &mut QualifiedRule, sort: bool) {
  match &mut rule.prelude {
    QualifiedRulePrelude::SelectorList(list) => process_selector_list(list, sort),
    QualifiedRulePrelude::RelativeSelectorList(list) => process_relative_selector_list(list, sort),
    _ => {}
  }
}

fn process_selector_list(list: &mut SelectorList, sort_lists: bool) {
  for complex in &mut list.children {
    process_complex_selector(complex, sort_lists);
  }

  dedupe_complex_selectors(&mut list.children);

  if sort_lists {
    sort_complex_selectors(&mut list.children);
  }
}

fn process_relative_selector_list(list: &mut RelativeSelectorList, sort_lists: bool) {
  for relative in &mut list.children {
    if let Some(combinator) = &mut relative.combinator {
      process_combinator(combinator);
    }
    process_complex_selector(&mut relative.selector, sort_lists);
  }

  dedupe_relative_selectors(&mut list.children);

  if sort_lists {
    sort_relative_selectors(&mut list.children);
  }
}

fn process_complex_selector(selector: &mut ComplexSelector, sort_nested_lists: bool) {
  let child_count = selector.children.len();
  for child in &mut selector.children {
    match child {
      ComplexSelectorChildren::CompoundSelector(compound) => {
        let is_simple = child_count == 1;
        process_compound_selector(compound, is_simple, sort_nested_lists);
      }
      ComplexSelectorChildren::Combinator(combinator) => {
        process_combinator(combinator);
      }
    }
  }
}

fn process_combinator(_combinator: &mut swc_core::css::ast::Combinator) {
  // The swc AST stores the combinator type as an enum, which already omits
  // redundant whitespace; no additional work is required here.
}

fn process_compound_selector(
  compound: &mut CompoundSelector,
  is_simple: bool,
  sort_nested_lists: bool,
) {
  if let Some(type_selector) = &mut compound.type_selector {
    match type_selector.as_mut() {
      TypeSelector::TagName(tag) => process_tag_selector(tag, is_simple),
      TypeSelector::Universal(_) => {
        if should_remove_universal(compound) {
          compound.type_selector = None;
        }
      }
    }
  }

  for subclass in &mut compound.subclass_selectors {
    process_subclass_selector(subclass, sort_nested_lists);
  }
}

fn should_remove_universal(compound: &CompoundSelector) -> bool {
  if !compound.subclass_selectors.is_empty() {
    return true;
  }

  if compound.nesting_selector.is_some() {
    return true;
  }

  false
}

fn process_tag_selector(tag: &mut swc_core::css::ast::TagNameSelector, is_simple: bool) {
  let original = tag.name.value.value.to_string();
  let lower = original.to_lowercase();

  if !is_simple {
    return;
  }

  for (source, replacement) in TAG_REPLACEMENTS {
    if lower == *source {
      tag.name.value.value = Atom::from(*replacement);
      tag.name.value.raw = None;
      return;
    }
  }
}

fn process_subclass_selector(subclass: &mut SubclassSelector, sort_nested_lists: bool) {
  match subclass {
    SubclassSelector::Attribute(attribute) => process_attribute_selector(attribute),
    SubclassSelector::PseudoClass(pseudo) => {
      process_pseudo_class_selector(pseudo, sort_nested_lists)
    }
    SubclassSelector::PseudoElement(pseudo) => {
      if let Some(mut converted) = convert_pseudo_element_to_class(pseudo) {
        process_pseudo_class_selector(&mut converted, sort_nested_lists);
        *subclass = SubclassSelector::PseudoClass(converted);
      } else {
        process_pseudo_element_selector(pseudo);
      }
    }
    SubclassSelector::Id(_) | SubclassSelector::Class(_) => {}
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

fn normalize_attribute_string(str_value: &mut swc_core::css::ast::Str) {
  let mut value = str_value.value.to_string();
  if value.contains("\\\n") {
    value = value.replace("\\\n", "");
  }
  if value.contains("\\r") {
    value = value.replace("\\r", "");
  }
  if value.contains("\\f") {
    value = value.replace("\\f", "");
  }

  str_value.value = Atom::from(value.trim().to_string());

  if let Some(raw) = &str_value.raw {
    let mut raw_value = raw.to_string();
    if raw_value.contains("\\\n") {
      raw_value = raw_value.replace("\\\n", "");
    }
    if raw_value.contains("\\r") {
      raw_value = raw_value.replace("\\r", "");
    }
    if raw_value.contains("\\f") {
      raw_value = raw_value.replace("\\f", "");
    }
    str_value.raw = Some(Atom::from(raw_value.trim().to_string()));
  }
}

fn process_pseudo_class_selector(selector: &mut PseudoClassSelector, sort_nested_lists: bool) {
  let name = selector.name.value.to_string();
  let lower = name.to_lowercase();

  if selector.children.is_some() {
    if handle_nth_replacements(selector, &lower) {
      return;
    }
  }

  if let Some(children) = &mut selector.children {
    for child in children {
      match child {
        PseudoClassSelectorChildren::SelectorList(list) => {
          process_selector_list(list, false);
        }
        PseudoClassSelectorChildren::RelativeSelectorList(list) => {
          process_relative_selector_list(list, false);
        }
        PseudoClassSelectorChildren::ForgivingSelectorList(list) => {
          for selector in &mut list.children {
            if let swc_core::css::ast::ForgivingComplexSelector::ComplexSelector(selector) =
              selector
            {
              process_complex_selector(selector, sort_nested_lists);
            }
          }
          dedupe_forgiving_selectors(&mut list.children);
        }
        PseudoClassSelectorChildren::ForgivingRelativeSelectorList(list) => {
          for selector in &mut list.children {
            if let swc_core::css::ast::ForgivingRelativeSelector::RelativeSelector(relative) =
              selector
            {
              if let Some(combinator) = &mut relative.combinator {
                process_combinator(combinator);
              }
              process_complex_selector(&mut relative.selector, sort_nested_lists);
            }
          }
        }
        PseudoClassSelectorChildren::ComplexSelector(selector) => {
          process_complex_selector(selector, sort_nested_lists);
        }
        PseudoClassSelectorChildren::CompoundSelectorList(list) => {
          for compound in &mut list.children {
            process_compound_selector(compound, false, sort_nested_lists);
          }
        }
        PseudoClassSelectorChildren::CompoundSelector(compound) => {
          process_compound_selector(compound, false, sort_nested_lists);
        }
        PseudoClassSelectorChildren::PreservedToken(_)
        | PseudoClassSelectorChildren::AnPlusB(_)
        | PseudoClassSelectorChildren::Ident(_)
        | PseudoClassSelectorChildren::Str(_)
        | PseudoClassSelectorChildren::Delimiter(_) => {}
      }
    }
  }

  if matches!(
    PSEUDO_ELEMENTS.iter().find(|&&pseudo| pseudo == lower),
    Some(_)
  ) {
    if selector.name.raw.is_none() {
      selector.name.raw = Some(Atom::from(format!(":{}", selector.name.value)));
    } else {
      let raw = selector.name.raw.as_ref().unwrap().to_string();
      if raw.starts_with("::") {
        selector.name.raw = Some(Atom::from(raw.trim_start_matches(':')));
      }
    }
  }
}

fn handle_nth_replacements(selector: &mut PseudoClassSelector, lower_name: &str) -> bool {
  let mut replaced = false;

  if let Some((_, replacement)) = PSEUDO_REPLACEMENTS
    .iter()
    .find(|(source, _)| lower_name == *source)
  {
    if let Some(children) = &mut selector.children {
      if children.len() == 1 {
        match &mut children[0] {
          PseudoClassSelectorChildren::AnPlusB(an_plus_b) => match an_plus_b {
            AnPlusB::Ident(ident) => {
              if ident.value.eq_ignore_ascii_case("even") {
                ident.value = Atom::from("2n");
                ident.raw = None;
              }
            }
            AnPlusB::AnPlusBNotation(notation) => {
              let a = notation.a.unwrap_or(0);
              let b = notation.b.unwrap_or(0);

              if a == 0 && b == 1 {
                selector.name.value = Atom::from(*replacement);
                selector.name.raw = None;
                selector.children = None;
                replaced = true;
              } else if a == 2 && b == 1 {
                let ident = Ident {
                  span: notation.span,
                  value: Atom::from("odd"),
                  raw: None,
                };
                *an_plus_b = AnPlusB::Ident(ident);
                replaced = true;
              }
            }
          },
          PseudoClassSelectorChildren::Ident(ident) => {
            if ident.value.eq_ignore_ascii_case("even") {
              ident.value = Atom::from("2n");
              ident.raw = None;
              replaced = true;
            }
          }
          _ => {}
        }
      }
    }
  }

  replaced
}

fn process_pseudo_element_selector(selector: &mut PseudoElementSelector) {
  let name = selector.name.value.to_string();
  let lower = name.to_lowercase();

  if PSEUDO_ELEMENTS.iter().any(|&pseudo| pseudo == lower) {
    let updated_raw = if let Some(raw) = &selector.name.raw {
      let raw_text = raw.to_string();
      if raw_text.starts_with("::") {
        format!(":{}", &raw_text[2..])
      } else if raw_text.starts_with(':') {
        raw_text
      } else {
        format!(":{}", raw_text)
      }
    } else {
      format!(":{}", selector.name.value)
    };

    selector.name.raw = Some(Atom::from(updated_raw));
  }
}

fn convert_pseudo_element_to_class(pseudo: &PseudoElementSelector) -> Option<PseudoClassSelector> {
  let lower = pseudo.name.value.to_string().to_lowercase();

  if !PSEUDO_ELEMENTS.iter().any(|&element| element == lower) {
    return None;
  }

  let raw_name = if let Some(raw) = &pseudo.name.raw {
    let raw_text = raw.to_string();
    if raw_text.starts_with("::") {
      format!(":{}", &raw_text[2..])
    } else if raw_text.starts_with(':') {
      raw_text
    } else {
      format!(":{}", raw_text)
    }
  } else {
    format!(":{}", pseudo.name.value)
  };

  Some(PseudoClassSelector {
    span: pseudo.span,
    name: Ident {
      span: pseudo.name.span,
      value: pseudo.name.value.clone(),
      raw: Some(Atom::from(raw_name)),
    },
    children: None,
  })
}

fn dedupe_complex_selectors(selectors: &mut Vec<ComplexSelector>) {
  let mut seen = IndexSet::new();
  selectors.retain(|selector| {
    let serialized = selector_stringifier::serialize_complex_selector(selector);
    if seen.contains(&serialized) {
      false
    } else {
      seen.insert(serialized);
      true
    }
  });
}

fn dedupe_relative_selectors(selectors: &mut Vec<RelativeSelector>) {
  let mut seen = IndexSet::new();
  selectors.retain(|selector| {
    let serialized = selector_stringifier::serialize_relative_selector(selector);
    if seen.contains(&serialized) {
      false
    } else {
      seen.insert(serialized);
      true
    }
  });
}

fn dedupe_forgiving_selectors(selectors: &mut Vec<swc_core::css::ast::ForgivingComplexSelector>) {
  let mut seen = IndexSet::new();
  selectors.retain(|selector| match selector {
    swc_core::css::ast::ForgivingComplexSelector::ComplexSelector(selector) => {
      let serialized = selector_stringifier::serialize_complex_selector(selector);
      if seen.contains(&serialized) {
        false
      } else {
        seen.insert(serialized);
        true
      }
    }
    swc_core::css::ast::ForgivingComplexSelector::ListOfComponentValues(_) => true,
  });
}

fn sort_complex_selectors(selectors: &mut Vec<ComplexSelector>) {
  selectors.sort_by(|a, b| {
    selector_stringifier::serialize_complex_selector(a)
      .cmp(&selector_stringifier::serialize_complex_selector(b))
  });
}

fn sort_relative_selectors(selectors: &mut Vec<RelativeSelector>) {
  selectors.sort_by(|a, b| {
    selector_stringifier::serialize_relative_selector(a)
      .cmp(&selector_stringifier::serialize_relative_selector(b))
  });
}

fn can_unquote(value: &str) -> bool {
  if value.is_empty() || value == "-" {
    return false;
  }

  let mut sanitized = String::new();
  let mut chars = value.chars().peekable();
  while let Some(ch) = chars.next() {
    if ch == '\\' {
      let mut hex = String::new();
      while let Some(next) = chars.peek() {
        if next.is_ascii_hexdigit() && hex.len() < 6 {
          hex.push(*next);
          chars.next();
        } else {
          break;
        }
      }

      if !hex.is_empty() {
        if matches!(
          chars.peek(),
          Some(' ') | Some('\t') | Some('\n') | Some('\r') | Some('\u{000C}')
        ) {
          chars.next();
        }
        sanitized.push('a');
        continue;
      }

      if chars.next().is_some() {
        sanitized.push('a');
        continue;
      }
    }

    sanitized.push(ch);
  }

  if sanitized.starts_with("--") {
    return false;
  }

  let mut iter = sanitized.chars();
  if let Some(first) = iter.next() {
    if first.is_ascii_digit() {
      return false;
    }
    if first == '-' {
      if let Some(second) = iter.next() {
        if second.is_ascii_digit() {
          return false;
        }
      }
    }
  }

  sanitized.chars().all(|ch| {
    let code = ch as u32;
    match code {
      0x0000..=0x002C => false,
      0x002E..=0x002F => false,
      0x003A..=0x0040 => false,
      0x005B..=0x005E => false,
      0x0060 => false,
      0x007B..=0x009F => false,
      _ => true,
    }
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::postcss::{
    transform::{TransformContext, TransformCssOptions},
    utils::selector_stringifier,
  };
  use swc_core::common::{FileName, SourceMap, input::StringInput};
  use swc_core::css::codegen::{CodeGenerator, CodegenConfig, Emit, writer::basic::BasicCssWriter};
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

  fn run_plugin_stylesheet(css: &str) -> Stylesheet {
    let mut stylesheet = parse_stylesheet(css);
    let options = TransformCssOptions::default();
    let mut ctx = TransformContext::new(&options);
    let plugin = minify_selectors(true);
    plugin.run(&mut stylesheet, &mut ctx);
    stylesheet
  }

  fn first_qualified_rule(stylesheet: &Stylesheet) -> &QualifiedRule {
    stylesheet
      .rules
      .iter()
      .find_map(|rule| match rule {
        Rule::QualifiedRule(rule) => Some(rule),
        _ => None,
      })
      .expect("expected a qualified rule")
  }

  fn first_selector_list(rule: &QualifiedRule) -> &SelectorList {
    match &rule.prelude {
      QualifiedRulePrelude::SelectorList(list) => list,
      _ => panic!("expected selector list"),
    }
  }

  fn selector_texts(list: &SelectorList) -> Vec<String> {
    list
      .children
      .iter()
      .map(selector_stringifier::serialize_complex_selector)
      .collect()
  }

  fn first_compound_selector(list: &SelectorList) -> &CompoundSelector {
    list
      .children
      .first()
      .and_then(|complex| {
        complex.children.iter().find_map(|child| {
          if let ComplexSelectorChildren::CompoundSelector(compound) = child {
            Some(compound)
          } else {
            None
          }
        })
      })
      .expect("expected first compound selector")
  }

  fn first_pseudo_class(compound: &CompoundSelector) -> &PseudoClassSelector {
    compound
      .subclass_selectors
      .iter()
      .find_map(|selector| match selector {
        SubclassSelector::PseudoClass(pseudo) => Some(pseudo),
        _ => None,
      })
      .expect("expected pseudo class selector")
  }

  #[test]
  fn removes_duplicate_selectors() {
    let stylesheet = run_plugin_stylesheet(".a, .a, .b { color: red; }");
    let list = first_selector_list(first_qualified_rule(&stylesheet));
    assert_eq!(selector_texts(list), vec![".a", ".b"]);
  }

  #[test]
  fn sorts_selectors() {
    let stylesheet = run_plugin_stylesheet(".b, .a { color: blue; }");
    let list = first_selector_list(first_qualified_rule(&stylesheet));
    assert_eq!(selector_texts(list), vec![".a", ".b"]);
  }

  #[test]
  fn converts_nth_child_one_to_first_child() {
    let stylesheet = run_plugin_stylesheet(":nth-child(1) { color: red; }");
    let compound = first_compound_selector(first_selector_list(first_qualified_rule(&stylesheet)));
    let pseudo = first_pseudo_class(compound);
    assert_eq!(pseudo.name.value.as_ref(), "first-child");
    assert!(pseudo.children.is_none());
  }

  #[test]
  fn converts_even_to_two_n() {
    let stylesheet = run_plugin_stylesheet(":nth-child(even) { color: blue; }");
    let compound = first_compound_selector(first_selector_list(first_qualified_rule(&stylesheet)));
    let pseudo = first_pseudo_class(compound);
    let children = pseudo.children.as_ref().expect("expected children");
    assert_eq!(children.len(), 1);
    match &children[0] {
      PseudoClassSelectorChildren::AnPlusB(AnPlusB::Ident(ident)) => {
        assert_eq!(ident.value.as_ref(), "2n");
      }
      other => panic!("unexpected child: {:?}", other),
    }
  }

  #[test]
  fn converts_two_n_plus_one_to_odd() {
    let stylesheet = run_plugin_stylesheet(":nth-child(2n+1) { color: green; }");
    let compound = first_compound_selector(first_selector_list(first_qualified_rule(&stylesheet)));
    let pseudo = first_pseudo_class(compound);
    let children = pseudo.children.as_ref().expect("expected children");
    assert_eq!(children.len(), 1);
    match &children[0] {
      PseudoClassSelectorChildren::AnPlusB(AnPlusB::Ident(ident)) => {
        assert_eq!(ident.value.as_ref(), "odd");
      }
      other => panic!("unexpected child: {:?}", other),
    }
  }

  #[test]
  fn preserves_an_plus_b_coefficients() {
    let stylesheet = run_plugin_stylesheet("p:nth-of-type(n+2) { color: green; }");
    let compound = first_compound_selector(first_selector_list(first_qualified_rule(&stylesheet)));
    let pseudo = first_pseudo_class(compound);
    let children = pseudo.children.as_ref().expect("expected children");
    assert_eq!(children.len(), 1);
    match &children[0] {
      PseudoClassSelectorChildren::AnPlusB(AnPlusB::AnPlusBNotation(notation)) => {
        assert_eq!(notation.a, Some(1));
        assert_eq!(notation.b, Some(2));
      }
      other => panic!("unexpected child: {:?}", other),
    }

    let serialized_rule = {
      let mut output = String::new();
      let writer = BasicCssWriter::new(&mut output, None, Default::default());
      let mut generator = CodeGenerator::new(writer, CodegenConfig { minify: true });
      generator.emit(first_qualified_rule(&stylesheet)).unwrap();
      output
    };

    assert!(
      serialized_rule.contains("nth-of-type(n+2)"),
      "serialized rule should preserve coefficient: {}",
      serialized_rule
    );
  }

  #[test]
  fn unquotes_attribute_values_when_safe() {
    let stylesheet = run_plugin_stylesheet("[data-test=\"foo\"] { color: red; }");
    let compound = first_compound_selector(first_selector_list(first_qualified_rule(&stylesheet)));
    let attribute = compound
      .subclass_selectors
      .iter()
      .find_map(|selector| match selector {
        SubclassSelector::Attribute(attribute) => Some(attribute),
        _ => None,
      })
      .expect("expected attribute selector");

    match attribute.value.as_ref() {
      Some(AttributeSelectorValue::Ident(ident)) => {
        assert_eq!(ident.value.as_ref(), "foo");
      }
      other => panic!("unexpected attribute value: {:?}", other),
    }
  }

  #[test]
  fn removes_universal_when_followed_by_class() {
    let stylesheet = run_plugin_stylesheet("*.foo { color: red; }");
    let compound = first_compound_selector(first_selector_list(first_qualified_rule(&stylesheet)));
    assert!(compound.type_selector.is_none());
  }

  #[test]
  fn converts_known_pseudo_elements_to_pseudo_classes() {
    let stylesheet = run_plugin_stylesheet("::before { color: red; }");
    let compound = first_compound_selector(first_selector_list(first_qualified_rule(&stylesheet)));

    assert!(
      compound
        .subclass_selectors
        .iter()
        .all(|selector| !matches!(selector, SubclassSelector::PseudoElement(_)))
    );

    let pseudo = first_pseudo_class(compound);
    assert_eq!(pseudo.name.value.as_ref(), "before");
    assert_eq!(
      pseudo
        .name
        .raw
        .as_ref()
        .map(|raw| raw.as_ref())
        .expect("expected raw name"),
      ":before"
    );
  }

  #[test]
  fn dedupes_nested_selectors_inside_pseudos() {
    let stylesheet = run_plugin_stylesheet(":is(.a, .a, .b) { color: red; }");
    let compound = first_compound_selector(first_selector_list(first_qualified_rule(&stylesheet)));
    let pseudo = first_pseudo_class(compound);
    let children = pseudo.children.as_ref().expect("expected children");

    let list = match &children[0] {
      PseudoClassSelectorChildren::ForgivingSelectorList(list) => list,
      other => panic!("unexpected pseudo child: {:?}", other),
    };

    let selectors: Vec<String> = list
      .children
      .iter()
      .filter_map(|selector| match selector {
        swc_core::css::ast::ForgivingComplexSelector::ComplexSelector(selector) => {
          Some(selector_stringifier::serialize_complex_selector(selector))
        }
        swc_core::css::ast::ForgivingComplexSelector::ListOfComponentValues(_) => None,
      })
      .collect();

    assert_eq!(selectors, vec![".a", ".b"]);
  }
}
