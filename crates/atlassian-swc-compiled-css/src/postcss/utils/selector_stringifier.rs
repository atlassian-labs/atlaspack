use std::sync::Arc;

use swc_core::common::{FileName, SourceMap, input::StringInput};
use swc_core::css::ast::{
  AnPlusB, AttributeSelector, AttributeSelectorMatcherValue, AttributeSelectorValue, ClassSelector,
  Combinator, CombinatorValue, ComplexSelector, ComplexSelectorChildren, ComponentValue,
  CompoundSelector, CustomHighlightName, Delimiter, DelimiterValue, ForgivingComplexSelector,
  ForgivingRelativeSelector, ForgivingRelativeSelectorList, ForgivingSelectorList, Ident,
  ListOfComponentValues, Namespace, NamespacePrefix, PseudoClassSelector,
  PseudoClassSelectorChildren, PseudoElementSelector, PseudoElementSelectorChildren,
  QualifiedRulePrelude, RelativeSelector, RelativeSelectorList, Rule, SelectorList, Str,
  SubclassSelector, TokenAndSpan, TypeSelector, UniversalSelector, WqName,
};
use swc_core::css::codegen::{CodeGenerator, CodegenConfig, Emit, writer::basic::BasicCssWriter};
use swc_core::css::parser::{parse_string_input, parser::ParserConfig};

/// Serialize a selector list using postcss-minify-selectors style whitespace semantics.
pub fn serialize_selector_list(list: &SelectorList) -> String {
  list
    .children
    .iter()
    .map(serialize_complex_selector)
    .collect::<Vec<_>>()
    .join(",")
}

/// Serialize a relative selector list using postcss-minify-selectors semantics.
pub fn serialize_relative_selector_list(list: &RelativeSelectorList) -> String {
  list
    .children
    .iter()
    .map(serialize_relative_selector)
    .collect::<Vec<_>>()
    .join(",")
}

/// Serialize a forgiving selector list, skipping invalid selectors.
pub fn serialize_forgiving_selector_list(list: &ForgivingSelectorList) -> String {
  list
    .children
    .iter()
    .filter_map(|selector| match selector {
      ForgivingComplexSelector::ComplexSelector(selector) => {
        Some(serialize_complex_selector(selector))
      }
      ForgivingComplexSelector::ListOfComponentValues(list) => {
        serialize_list_of_component_values(list)
      }
    })
    .collect::<Vec<_>>()
    .join(",")
}

/// Serialize a forgiving relative selector list.
pub fn serialize_forgiving_relative_selector_list(list: &ForgivingRelativeSelectorList) -> String {
  list
    .children
    .iter()
    .filter_map(|selector| match selector {
      ForgivingRelativeSelector::RelativeSelector(selector) => {
        Some(serialize_relative_selector(selector))
      }
      ForgivingRelativeSelector::ListOfComponentValues(list) => {
        serialize_list_of_component_values(list)
      }
    })
    .collect::<Vec<_>>()
    .join(",")
}

/// Serialize a single complex selector without adding extra whitespace.
pub fn serialize_complex_selector(selector: &ComplexSelector) -> String {
  let mut out = String::new();
  for child in &selector.children {
    match child {
      ComplexSelectorChildren::CompoundSelector(compound) => {
        out.push_str(&serialize_compound_selector(compound))
      }
      ComplexSelectorChildren::Combinator(combinator) => {
        out.push_str(&serialize_combinator(combinator))
      }
    }
  }
  out
}

/// Serialize a relative selector (used in :has/:is contexts).
pub fn serialize_relative_selector(selector: &RelativeSelector) -> String {
  let mut out = String::new();
  if let Some(combinator) = &selector.combinator {
    out.push_str(&serialize_combinator(combinator));
  }
  out.push_str(&serialize_complex_selector(&selector.selector));
  out
}

fn serialize_combinator(combinator: &Combinator) -> &'static str {
  match combinator.value {
    CombinatorValue::Descendant => " ",
    CombinatorValue::Child => ">",
    CombinatorValue::NextSibling => "+",
    CombinatorValue::LaterSibling => "~",
    CombinatorValue::Column => "||",
  }
}

fn serialize_compound_selector(compound: &CompoundSelector) -> String {
  let mut out = String::new();
  if compound.nesting_selector.is_some() {
    out.push('&');
  }
  if let Some(selector) = &compound.type_selector {
    out.push_str(&serialize_type_selector(selector));
  }
  for subclass in &compound.subclass_selectors {
    out.push_str(&serialize_subclass_selector(subclass));
  }
  out
}

fn serialize_type_selector(selector: &TypeSelector) -> String {
  match selector {
    TypeSelector::TagName(tag) => serialize_tag_name(tag),
    TypeSelector::Universal(universal) => serialize_universal(universal),
  }
}

fn serialize_tag_name(tag: &swc_core::css::ast::TagNameSelector) -> String {
  format!(
    "{}{}",
    serialize_namespace_prefix(tag.name.prefix.as_ref()),
    serialize_ident(&tag.name.value)
  )
}

fn serialize_universal(selector: &UniversalSelector) -> String {
  format!("{}*", serialize_namespace_prefix(selector.prefix.as_ref()))
}

fn serialize_namespace_prefix(prefix: Option<&NamespacePrefix>) -> String {
  let Some(prefix) = prefix else {
    return String::new();
  };

  let mut out = String::new();
  match &prefix.namespace {
    Some(Namespace::Named(named)) => out.push_str(&serialize_ident(&named.name)),
    Some(Namespace::Any(_)) => out.push('*'),
    None => {}
  }
  out.push('|');
  out
}

fn serialize_subclass_selector(selector: &SubclassSelector) -> String {
  match selector {
    SubclassSelector::Class(class_selector) => serialize_class_selector(class_selector),
    SubclassSelector::Id(id_selector) => serialize_id_selector(id_selector),
    SubclassSelector::Attribute(attribute) => serialize_attribute_selector(attribute),
    SubclassSelector::PseudoClass(pseudo) => serialize_pseudo_class_selector(pseudo),
    SubclassSelector::PseudoElement(pseudo) => serialize_pseudo_element_selector(pseudo),
  }
}

fn serialize_class_selector(selector: &ClassSelector) -> String {
  format!(".{}", serialize_ident(&selector.text))
}

fn serialize_id_selector(selector: &swc_core::css::ast::IdSelector) -> String {
  format!("#{}", serialize_ident(&selector.text))
}

fn serialize_attribute_selector(selector: &AttributeSelector) -> String {
  let mut out = String::new();
  out.push('[');
  out.push_str(&serialize_wq_name(&selector.name));
  if let Some(matcher) = &selector.matcher {
    out.push_str(match matcher.value {
      AttributeSelectorMatcherValue::Equals => "=",
      AttributeSelectorMatcherValue::Tilde => "~=",
      AttributeSelectorMatcherValue::Bar => "|=",
      AttributeSelectorMatcherValue::Caret => "^=",
      AttributeSelectorMatcherValue::Dollar => "$=",
      AttributeSelectorMatcherValue::Asterisk => "*=",
    });
  }
  if let Some(value) = &selector.value {
    out.push_str(&serialize_attribute_value(value));
  }
  if let Some(modifier) = &selector.modifier {
    out.push(' ');
    out.push_str(&serialize_ident(&modifier.value));
  }
  out.push(']');
  out
}

fn serialize_attribute_value(value: &AttributeSelectorValue) -> String {
  match value {
    AttributeSelectorValue::Str(str) => serialize_string(str),
    AttributeSelectorValue::Ident(ident) => serialize_ident(ident),
  }
}

fn serialize_wq_name(name: &WqName) -> String {
  format!(
    "{}{}",
    serialize_namespace_prefix(name.prefix.as_ref()),
    serialize_ident(&name.value)
  )
}

fn serialize_pseudo_class_selector(selector: &PseudoClassSelector) -> String {
  let mut out = String::new();
  out.push_str(&serialize_prefixed_ident(":", &selector.name));
  if let Some(children) = &selector.children {
    out.push('(');
    out.push_str(&serialize_pseudo_class_children(children));
    out.push(')');
  }
  out
}

fn serialize_pseudo_class_children(children: &[PseudoClassSelectorChildren]) -> String {
  let mut parts: Vec<String> = Vec::new();
  for child in children {
    match child {
      PseudoClassSelectorChildren::SelectorList(list) => parts.push(serialize_selector_list(list)),
      PseudoClassSelectorChildren::RelativeSelectorList(list) => {
        parts.push(serialize_relative_selector_list(list))
      }
      PseudoClassSelectorChildren::ForgivingSelectorList(list) => {
        parts.push(serialize_forgiving_selector_list(list))
      }
      PseudoClassSelectorChildren::ForgivingRelativeSelectorList(list) => {
        parts.push(serialize_forgiving_relative_selector_list(list))
      }
      PseudoClassSelectorChildren::CompoundSelectorList(list) => {
        let serialized = list
          .children
          .iter()
          .map(serialize_compound_selector)
          .collect::<Vec<_>>()
          .join(",");
        parts.push(serialized);
      }
      PseudoClassSelectorChildren::ComplexSelector(selector) => {
        parts.push(serialize_complex_selector(selector));
      }
      PseudoClassSelectorChildren::CompoundSelector(selector) => {
        parts.push(serialize_compound_selector(selector));
      }
      PseudoClassSelectorChildren::AnPlusB(value) => {
        parts.push(serialize_an_plus_b(value));
      }
      PseudoClassSelectorChildren::Ident(ident) => {
        parts.push(serialize_ident(ident));
      }
      PseudoClassSelectorChildren::Str(value) => parts.push(serialize_string(value)),
      PseudoClassSelectorChildren::Delimiter(delimiter) => {
        parts.push(serialize_delimiter(delimiter));
      }
      PseudoClassSelectorChildren::PreservedToken(token) => {
        if let Some(serialized) = serialize_token(token) {
          parts.push(serialized);
        }
      }
    }
  }
  parts.join("")
}

fn serialize_pseudo_element_selector(selector: &PseudoElementSelector) -> String {
  let mut out = String::new();
  out.push_str(&serialize_prefixed_ident("::", &selector.name));
  if let Some(children) = &selector.children {
    let serialized_children = children
      .iter()
      .filter_map(|child| match child {
        PseudoElementSelectorChildren::CompoundSelector(selector) => {
          Some(serialize_compound_selector(selector))
        }
        PseudoElementSelectorChildren::Ident(ident) => Some(serialize_ident(ident)),
        PseudoElementSelectorChildren::CustomHighlightName(name) => {
          Some(serialize_custom_highlight_name(name))
        }
        PseudoElementSelectorChildren::PreservedToken(token) => serialize_token(token),
      })
      .collect::<Vec<_>>()
      .join("");
    if !serialized_children.is_empty() {
      out.push('(');
      out.push_str(&serialized_children);
      out.push(')');
    }
  }
  out
}

fn serialize_an_plus_b(value: &AnPlusB) -> String {
  match value {
    AnPlusB::Ident(ident) => serialize_ident(ident),
    AnPlusB::AnPlusBNotation(notation) => {
      let mut out = String::new();
      let sanitized_a_raw = notation.a_raw.as_ref().map(|raw| {
        raw
          .as_ref()
          .chars()
          .filter(|c| !c.is_whitespace())
          .collect::<String>()
      });
      if let Some(a_raw) = sanitized_a_raw.as_ref().filter(|raw| !raw.is_empty()) {
        // SWC exposes the coefficient raw without the trailing `n` (e.g. "1"),
        // while postcss-selector-parser retains the full "1n". Append `n` when
        // the raw value does not already include it to mirror Babel output.
        out.push_str(a_raw);
        if !a_raw.to_ascii_lowercase().contains('n') {
          out.push('n');
        }
      } else if let Some(a) = notation.a {
        match a {
          0 => {}
          1 => out.push_str("n"),
          -1 => out.push_str("-n"),
          _ => {
            out.push_str(&a.to_string());
            out.push('n');
          }
        }
      }
      let sanitized_b_raw = notation.b_raw.as_ref().map(|raw| {
        raw
          .as_ref()
          .chars()
          .filter(|c| !c.is_whitespace())
          .collect::<String>()
      });
      if let Some(b_raw) = sanitized_b_raw.as_ref().filter(|raw| !raw.is_empty()) {
        // SWC keeps whitespace in the raw `b` component; remove it to mirror
        // postcss-selector-parser output and avoid emitting duplicate operators.
        if !out.is_empty() && !b_raw.starts_with(['+', '-']) {
          out.push('+');
        }
        out.push_str(b_raw);
      } else if let Some(b) = notation.b {
        if b != 0 {
          if !out.is_empty() && b > 0 {
            out.push('+');
          }
          out.push_str(&b.to_string());
        } else if out.is_empty() {
          out.push('0');
        }
      } else if out.is_empty() {
        out.push('0');
      }
      out
    }
  }
}

fn serialize_delimiter(delimiter: &Delimiter) -> String {
  match delimiter.value {
    DelimiterValue::Comma => ",".to_string(),
    DelimiterValue::Solidus => "/".to_string(),
    DelimiterValue::Semicolon => ";".to_string(),
  }
}

fn serialize_token(token: &TokenAndSpan) -> Option<String> {
  let mut output = String::new();
  {
    let writer = BasicCssWriter::new(&mut output, None, Default::default());
    let mut generator = CodeGenerator::new(writer, CodegenConfig { minify: true });
    if generator.emit(token).is_err() {
      return None;
    }
  }
  Some(output)
}

fn serialize_string(value: &Str) -> String {
  if let Some(raw) = &value.raw {
    raw.to_string()
  } else {
    format!("\"{}\"", value.value)
  }
}

fn serialize_ident(ident: &Ident) -> String {
  ident
    .raw
    .as_ref()
    .map(|raw| raw.to_string())
    .unwrap_or_else(|| ident.value.to_string())
}

fn serialize_prefixed_ident(prefix: &str, ident: &Ident) -> String {
  if let Some(raw) = &ident.raw {
    if raw.starts_with(prefix) {
      raw.to_string()
    } else {
      format!("{}{}", prefix, raw)
    }
  } else {
    format!("{}{}", prefix, ident.value)
  }
}

fn serialize_custom_highlight_name(name: &CustomHighlightName) -> String {
  name
    .raw
    .as_ref()
    .map(|raw| raw.to_string())
    .unwrap_or_else(|| name.value.to_string())
}

pub fn serialize_component_values(values: &[ComponentValue]) -> Option<String> {
  let mut output = String::new();
  {
    let writer = BasicCssWriter::new(&mut output, None, Default::default());
    let mut generator = CodeGenerator::new(writer, CodegenConfig { minify: true });
    for value in values {
      if generator.emit(value).is_err() {
        return None;
      }
    }
  }
  Some(output)
}

pub fn serialize_list_of_component_values(list: &ListOfComponentValues) -> Option<String> {
  serialize_component_values(&list.children)
}

pub fn parse_selector_list_from_component_values(
  list: &ListOfComponentValues,
) -> Option<SelectorList> {
  let raw = serialize_list_of_component_values(list)?;
  parse_selector_list_from_str(&raw)
}

pub fn parse_selector_list_from_str(selector: &str) -> Option<SelectorList> {
  let css = format!("{}{{}}", selector);
  let cm: Arc<SourceMap> = Default::default();
  let fm = cm.new_source_file(FileName::Custom("selector.css".into()).into(), css);
  let mut errors = vec![];
  let stylesheet = parse_string_input::<swc_core::css::ast::Stylesheet>(
    StringInput::from(&*fm),
    None,
    ParserConfig::default(),
    &mut errors,
  )
  .ok()?;
  if !errors.is_empty() {
    return None;
  }

  match stylesheet.rules.into_iter().next()? {
    Rule::QualifiedRule(rule) => match rule.prelude {
      QualifiedRulePrelude::SelectorList(list) => Some(list),
      QualifiedRulePrelude::RelativeSelectorList(_) => None,
      QualifiedRulePrelude::ListOfComponentValues(_) => None,
    },
    _ => None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn removes_spaces_around_child_combinator() {
    let list = parse_selector_list_from_str(".foo > *").expect("selector");
    assert_eq!(serialize_selector_list(&list), ".foo>*");
  }

  #[test]
  fn parses_nesting_selector() {
    assert!(parse_selector_list_from_str("&>*").is_some());
  }

  #[test]
  fn preserves_pseudo_class_colon() {
    let list = parse_selector_list_from_str("& > :first-child").expect("selector");
    assert_eq!(serialize_selector_list(&list), "&>:first-child");
  }
}
