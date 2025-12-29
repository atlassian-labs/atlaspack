use once_cell::sync::Lazy;
use regex::Regex;
use swc_core::common::Spanned;
use swc_core::css::ast::{
  AtRule, AtRuleName, AtRulePrelude, ComponentValue, Declaration, DeclarationName,
  ListOfComponentValues, QualifiedRule, QualifiedRulePrelude, Rule, SimpleBlock, Stylesheet, Token,
};
use swc_core::css::codegen::{CodeGenerator, CodegenConfig, Emit, writer::basic::BasicCssWriter};

use super::super::transform::{Plugin, TransformContext};
use crate::postcss::utils::selector_stringifier;

#[derive(Debug, Default, Clone, Copy)]
pub struct ExtractStyleSheets;

impl Plugin for ExtractStyleSheets {
  fn name(&self) -> &'static str {
    "extract-stylesheets"
  }

  fn run(&self, stylesheet: &mut Stylesheet, ctx: &mut TransformContext<'_>) {
    if stylesheet.rules.is_empty() {
      return;
    }

    for rule in &stylesheet.rules {
      let Some(serialized) = serialize_rule(rule) else {
        continue;
      };

      ctx.push_sheet(serialized);
      if std::env::var("COMPILED_CSS_TRACE").is_ok() {
        if let Some(last) = ctx.sheets.last() {
          eprintln!("[extract] sheet='{}'", last);
        }
      }
    }
  }
}

pub fn extract_stylesheets() -> ExtractStyleSheets {
  ExtractStyleSheets
}

static AT_RULE_SPACE_REGEX: Lazy<Regex> = Lazy::new(|| {
  Regex::new(r"@(?:(media|supports|container|document|-moz-document))\(")
    .expect("valid at-rule spacing regex")
});

fn serialize_rule(rule: &Rule) -> Option<String> {
  let serialized = match rule {
    Rule::QualifiedRule(rule) => serialize_qualified_rule(rule),
    Rule::AtRule(rule) => serialize_at_rule(rule),
    Rule::ListOfComponentValues(list) => serialize_list_of_component_values(list),
  }?;

  let normalized = AT_RULE_SPACE_REGEX
    .replace_all(&serialized, |caps: &regex::Captures| {
      format!("@{} (", &caps[1])
    })
    .into_owned();
  Some(normalize_block_value_spacing(&normalized))
}

fn serialize_qualified_rule(rule: &QualifiedRule) -> Option<String> {
  match &rule.prelude {
    QualifiedRulePrelude::SelectorList(list) => {
      let selectors = selector_stringifier::serialize_selector_list(list);
      let block = serialize_simple_block(&rule.block)?;
      Some(format!("{selectors}{block}"))
    }
    QualifiedRulePrelude::RelativeSelectorList(list) => {
      let selectors = selector_stringifier::serialize_relative_selector_list(list);
      let block = serialize_simple_block(&rule.block)?;
      Some(format!("{selectors}{block}"))
    }
    QualifiedRulePrelude::ListOfComponentValues(list) => {
      if let Some(parsed) = selector_stringifier::parse_selector_list_from_component_values(list) {
        let selectors = selector_stringifier::serialize_selector_list(&parsed);
        let block = serialize_simple_block(&rule.block)?;
        Some(format!("{selectors}{block}"))
      } else {
        let selectors = serialize_with_codegen(list)?;
        let block = serialize_simple_block(&rule.block)?;
        Some(format!("{selectors}{block}"))
      }
    }
  }
}

fn serialize_at_rule(at_rule: &AtRule) -> Option<String> {
  let mut out = String::new();
  out.push('@');
  out.push_str(&at_rule_name(&at_rule.name));

  if let Some(prelude) = &at_rule.prelude {
    let serialized = serialize_at_rule_prelude(prelude)?;
    if !serialized.is_empty() {
      out.push(' ');
      out.push_str(&serialized);
    }
  }

  if let Some(block) = &at_rule.block {
    out.push_str(&serialize_simple_block(block)?);
  } else {
    out.push(';');
  }

  Some(out)
}

fn serialize_simple_block(block: &SimpleBlock) -> Option<String> {
  let (open, close) = match block.name.token {
    Token::LBrace => ('{', '}'),
    Token::LBracket => ('[', ']'),
    Token::LParen => ('(', ')'),
    _ => ('{', '}'),
  };

  let mut out = String::new();
  out.push(open);
  let contents = serialize_component_values(&block.value)?;
  out.push_str(&contents);
  out.push(close);
  Some(out)
}

fn serialize_component_values(values: &[ComponentValue]) -> Option<String> {
  if values.is_empty() {
    return Some(String::new());
  }

  let mut parts: Vec<(&ComponentValue, String)> = Vec::with_capacity(values.len());
  for value in values {
    let serialized = match value {
      ComponentValue::QualifiedRule(rule) => serialize_qualified_rule(rule)?,
      ComponentValue::AtRule(rule) => serialize_at_rule(rule)?,
      ComponentValue::SimpleBlock(block) => serialize_simple_block(block)?,
      ComponentValue::ListOfComponentValues(list) => serialize_list_of_component_values(list)?,
      ComponentValue::Declaration(decl) => serialize_declaration(decl)?,
      _ => serialize_with_codegen(value)?,
    };
    parts.push((value, serialized));
  }

  let mut out = String::new();
  for index in 0..parts.len() {
    out.push_str(&parts[index].1);
  }
  Some(out)
}

fn serialize_list_of_component_values(list: &ListOfComponentValues) -> Option<String> {
  serialize_value_sequence(&list.children)
}

fn serialize_with_codegen<T>(node: &T) -> Option<String>
where
  T: Spanned,
  for<'writer> CodeGenerator<BasicCssWriter<'writer, &'writer mut String>>: Emit<T>,
{
  let mut output = String::new();
  {
    let writer = BasicCssWriter::new(&mut output, None, Default::default());
    let mut generator = CodeGenerator::new(writer, CodegenConfig { minify: false });
    if generator.emit(node).is_err() {
      return None;
    }
  }
  Some(output)
}

fn serialize_at_rule_prelude(prelude: &AtRulePrelude) -> Option<String> {
  serialize_with_codegen(prelude)
}

fn at_rule_name(name: &AtRuleName) -> String {
  match name {
    AtRuleName::Ident(ident) => ident.value.to_string(),
    AtRuleName::DashedIdent(ident) => ident.value.to_string(),
  }
}

fn serialize_declaration(declaration: &Declaration) -> Option<String> {
  let mut out = String::new();
  out.push_str(&declaration_name_to_string(&declaration.name));
  out.push(':');

  let value_text = serialize_value_sequence(&declaration.value)?;
  out.push_str(&value_text);

  if declaration.important.is_some() {
    out.push_str(" !important");
  }

  Some(out)
}

fn declaration_name_to_string(name: &DeclarationName) -> String {
  match name {
    DeclarationName::Ident(ident) => ident.value.to_string(),
    DeclarationName::DashedIdent(ident) => ident.value.to_string(),
  }
}

fn serialize_value_sequence(values: &[ComponentValue]) -> Option<String> {
  if values.is_empty() {
    return Some(String::new());
  }

  let mut parts: Vec<(ValueComponentKind, String)> = Vec::with_capacity(values.len());
  for value in values {
    let serialized = match value {
      ComponentValue::QualifiedRule(rule) => serialize_qualified_rule(rule)?,
      ComponentValue::AtRule(rule) => serialize_at_rule(rule)?,
      ComponentValue::SimpleBlock(block) => serialize_simple_block(block)?,
      ComponentValue::ListOfComponentValues(list) => serialize_list_of_component_values(list)?,
      _ => serialize_with_codegen(value)?,
    };
    let kind = classify_value_component(value);
    parts.push((kind, serialized));
  }

  let mut out = String::new();
  for index in 0..parts.len() {
    out.push_str(&parts[index].1);
    if index + 1 < parts.len()
      && value_components_need_space(
        parts[index].0,
        &parts[index].1,
        parts[index + 1].0,
        &parts[index + 1].1,
      )
    {
      out.push(' ');
    }
  }

  Some(out)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ValueComponentKind {
  Divider,
  Whitespace,
  Other,
}

fn classify_value_component(value: &ComponentValue) -> ValueComponentKind {
  match value {
    ComponentValue::PreservedToken(token) => match token.token {
      Token::Comma { .. } => ValueComponentKind::Divider,
      Token::Delim { value } if value == '/' || value == ',' => ValueComponentKind::Divider,
      Token::WhiteSpace { .. } => ValueComponentKind::Whitespace,
      _ => ValueComponentKind::Other,
    },
    _ => ValueComponentKind::Other,
  }
}

fn value_components_need_space(
  current_kind: ValueComponentKind,
  current_text: &str,
  next_kind: ValueComponentKind,
  next_text: &str,
) -> bool {
  if current_text.is_empty()
    || next_text.is_empty()
    || current_kind == ValueComponentKind::Whitespace
    || next_kind == ValueComponentKind::Whitespace
    || current_kind == ValueComponentKind::Divider
    || next_kind == ValueComponentKind::Divider
  {
    return false;
  }

  let prev_char = current_text.chars().rev().find(|c| !c.is_whitespace());
  let next_char = next_text.chars().find(|c| !c.is_whitespace());
  prev_char.is_some() && next_char.is_some()
}

pub(crate) fn normalize_block_value_spacing(input: &str) -> String {
  let mut output = String::with_capacity(input.len());
  let mut chars = input.chars().peekable();
  let mut inside_block = false;
  let mut calc_stack: Vec<bool> = Vec::new();
  let mut current_ident = String::new();

  while let Some(ch) = chars.next() {
    if ch == '{' {
      inside_block = true;
      calc_stack.clear();
      current_ident.clear();
    } else if ch == '}' {
      inside_block = false;
      calc_stack.clear();
      current_ident.clear();
    }

    if ch == '(' {
      let is_calc = current_ident.eq_ignore_ascii_case("calc");
      calc_stack.push(is_calc);
      current_ident.clear();
    } else if ch == ')' {
      calc_stack.pop();
    } else if ch.is_alphabetic() || ch == '-' {
      current_ident.push(ch);
    } else {
      current_ident.clear();
    }

    output.push(ch);
    if inside_block && ch == ',' {
      let inside_calc = calc_stack.iter().any(|flag| *flag);
      if inside_calc {
        if matches!(chars.peek(), Some(c) if !c.is_whitespace()) {
          output.push(' ');
        }
      } else {
        while matches!(chars.peek(), Some(c) if c.is_whitespace()) {
          chars.next();
        }
      }
      continue;
    }

    if inside_block && ch == ')' {
      if let Some(&next) = chars.peek() {
        if needs_space_after_token(next) {
          output.push(' ');
        }
      }
    }
  }

  output
}

fn needs_space_after_token(next: char) -> bool {
  matches!(next, 'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_') && !next.is_whitespace()
}

#[cfg(test)]
mod tests {
  use super::*;
  use swc_core::common::{FileName, SourceMap, input::StringInput};
  use swc_core::css::parser::{parse_string_input, parser::ParserConfig};

  fn parse_stylesheet(css: &str) -> Stylesheet {
    let cm: std::sync::Arc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("inline.css".into()).into(), css.into());
    let mut errors = vec![];
    parse_string_input::<Stylesheet>(
      StringInput::from(&*fm),
      None,
      ParserConfig::default(),
      &mut errors,
    )
    .expect("failed to parse css")
  }

  #[test]
  fn serializes_padding_shorthand_with_spacing() {
    let css = "._a{padding:5 var(--foo) 0 0}";
    let stylesheet = parse_stylesheet(css);
    assert_eq!(stylesheet.rules.len(), 1);
    let serialized = serialize_rule(&stylesheet.rules[0]).expect("serialize rule");
    assert_eq!(serialized, css);
  }

  #[test]
  fn normalizes_missing_space_after_var_value() {
    let raw = "._a{padding:5 var(--foo)0 0}";
    let normalized = normalize_block_value_spacing(raw);
    assert_eq!(normalized, "._a{padding:5 var(--foo) 0 0}");
  }

  #[test]
  fn trims_var_fallback_outside_calc() {
    let raw = "._a{box-shadow:var(--foo, 10px)}";
    let normalized = normalize_block_value_spacing(raw);
    assert_eq!(normalized, "._a{box-shadow:var(--foo,10px)}");
  }

  #[test]
  fn preserves_var_fallback_inside_calc() {
    let raw = "._a{height:calc(100vh - var(--foo, 10px))}";
    let normalized = normalize_block_value_spacing(raw);
    assert_eq!(normalized, raw);
  }
}
