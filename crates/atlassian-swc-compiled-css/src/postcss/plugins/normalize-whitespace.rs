use std::collections::HashMap;

use once_cell::sync::Lazy;
use regex::Regex;
use swc_core::atoms::Atom;
use swc_core::css::ast::{
  AtRule, ComponentValue, Declaration, Function, FunctionName, KeyframeBlock, QualifiedRule, Rule,
  SimpleBlock, Stylesheet, Token,
};

use super::super::transform::{Plugin, TransformContext};
use crate::postcss::plugins::expand_shorthands::types::{
  declaration_property_name, parse_value_to_components, serialize_component_values,
};

static IE_HACK_WHITESPACE: Lazy<Regex> =
  Lazy::new(|| Regex::new(r"(?i)\s*(\\9)\s*").expect("invalid ie hack regex"));

#[derive(Debug, Default, Clone, Copy)]
pub struct NormalizeWhitespace;

impl Plugin for NormalizeWhitespace {
  fn name(&self) -> &'static str {
    "postcss-normalize-whitespace"
  }

  fn run(&self, stylesheet: &mut Stylesheet, _ctx: &mut TransformContext<'_>) {
    let mut cache: HashMap<String, String> = HashMap::new();
    normalize_stylesheet(stylesheet, &mut cache);
  }
}

pub fn normalize_whitespace() -> NormalizeWhitespace {
  NormalizeWhitespace
}

type ValueCache = HashMap<String, String>;

fn normalize_stylesheet(stylesheet: &mut Stylesheet, cache: &mut ValueCache) {
  for rule in &mut stylesheet.rules {
    normalize_rule(rule, cache);
  }
}

fn normalize_rule(rule: &mut Rule, cache: &mut ValueCache) {
  match rule {
    Rule::QualifiedRule(rule) => normalize_qualified_rule(rule, cache),
    Rule::AtRule(rule) => normalize_at_rule(rule, cache),
    Rule::ListOfComponentValues(list) => {
      normalize_component_values(&mut list.children, false);
    }
  }
}

fn normalize_qualified_rule(rule: &mut QualifiedRule, cache: &mut ValueCache) {
  normalize_simple_block(&mut rule.block, cache);
}

fn normalize_at_rule(rule: &mut AtRule, cache: &mut ValueCache) {
  if let Some(block) = &mut rule.block {
    normalize_simple_block(block, cache);
  }
}

fn normalize_keyframe_block(block: &mut KeyframeBlock, cache: &mut ValueCache) {
  normalize_simple_block(&mut block.block, cache);
}

fn normalize_simple_block(block: &mut SimpleBlock, cache: &mut ValueCache) {
  let mut index = 0usize;
  while index < block.value.len() {
    if is_whitespace_component(&block.value[index]) {
      block.value.remove(index);
      continue;
    }

    match &mut block.value[index] {
      ComponentValue::Declaration(declaration) => {
        normalize_declaration(declaration, cache);
      }
      ComponentValue::QualifiedRule(rule) => normalize_qualified_rule(rule, cache),
      ComponentValue::AtRule(rule) => normalize_at_rule(rule, cache),
      ComponentValue::SimpleBlock(inner) => normalize_simple_block(inner, cache),
      ComponentValue::KeyframeBlock(block) => normalize_keyframe_block(block, cache),
      ComponentValue::ListOfComponentValues(list) => {
        normalize_component_values(&mut list.children, false);
      }
      ComponentValue::Function(function) => {
        normalize_function(function, false);
      }
      _ => {}
    }

    index += 1;
  }

  while matches!(block.value.last(), Some(component) if is_whitespace_component(component)) {
    block.value.pop();
  }
}

fn normalize_declaration(declaration: &mut Declaration, cache: &mut ValueCache) {
  if declaration.value.is_empty() {
    return;
  }

  let Some(original_value) = serialize_component_values(&declaration.value) else {
    return;
  };

  if original_value.trim().is_empty() {
    declaration.value.clear();
    return;
  }

  let normalized_value = if let Some(cached) = cache.get(&original_value) {
    cached.clone()
  } else {
    let normalized = normalize_value(&original_value);
    cache.insert(original_value.clone(), normalized.clone());
    normalized
  };

  let property_name = declaration_property_name(&declaration.name);
  let final_value = if property_name.starts_with("--") && normalized_value.is_empty() {
    " ".to_string()
  } else {
    normalized_value
  };

  declaration.value = parse_value_to_components(&final_value);
}

fn normalize_value(value: &str) -> String {
  let without_ie_hack_spacing = IE_HACK_WHITESPACE.replace_all(value, "$1").into_owned();

  let mut components = parse_value_to_components(&without_ie_hack_spacing);
  normalize_component_values(&mut components, false);

  serialize_component_values(&components).unwrap_or(without_ie_hack_spacing)
}

fn normalize_component_values(values: &mut Vec<ComponentValue>, inside_calc: bool) {
  let mut index = 0usize;
  while index < values.len() {
    match &mut values[index] {
      ComponentValue::PreservedToken(token) => {
        if let Token::WhiteSpace { value } = &mut token.token {
          if value.as_ref() != " " {
            *value = Atom::from(" ");
          }
        }
      }
      ComponentValue::Function(function) => {
        normalize_function(function, inside_calc);
      }
      ComponentValue::SimpleBlock(block) => {
        normalize_component_values(&mut block.value, inside_calc);
      }
      ComponentValue::ListOfComponentValues(list) => {
        normalize_component_values(&mut list.children, inside_calc);
      }
      _ => {}
    }

    if !inside_calc && is_divider(&values[index]) {
      remove_previous_whitespace(values, &mut index);
      remove_following_whitespace(values, index);
    }

    index += 1;
  }
}

fn normalize_function(function: &mut Function, parent_inside_calc: bool) {
  let name = function_name(&function.name);
  let lower = name.to_ascii_lowercase();
  let is_variable = matches!(lower.as_str(), "var" | "env" | "constant");
  let is_calc = lower == "calc";

  let should_trim = !(is_variable && parent_inside_calc);
  if should_trim {
    trim_leading_whitespace(&mut function.value);
    trim_trailing_whitespace(&mut function.value);
  }

  normalize_component_values(&mut function.value, parent_inside_calc || is_calc);
}

fn remove_previous_whitespace(values: &mut Vec<ComponentValue>, index: &mut usize) {
  while *index > 0 {
    if is_whitespace_component(&values[*index - 1]) {
      values.remove(*index - 1);
      *index -= 1;
    } else {
      break;
    }
  }
}

fn remove_following_whitespace(values: &mut Vec<ComponentValue>, index: usize) {
  while index + 1 < values.len() && is_whitespace_component(&values[index + 1]) {
    values.remove(index + 1);
  }
}

fn trim_leading_whitespace(values: &mut Vec<ComponentValue>) {
  while matches!(values.first(), Some(component) if is_whitespace_component(component)) {
    values.remove(0);
  }
}

fn trim_trailing_whitespace(values: &mut Vec<ComponentValue>) {
  while matches!(values.last(), Some(component) if is_whitespace_component(component)) {
    values.pop();
  }
}

fn function_name(name: &FunctionName) -> String {
  match name {
    FunctionName::Ident(ident) => ident.value.to_string(),
    FunctionName::DashedIdent(ident) => ident.value.to_string(),
  }
}

fn is_whitespace_component(component: &ComponentValue) -> bool {
  matches!(
      component,
      ComponentValue::PreservedToken(token)
          if matches!(token.token, Token::WhiteSpace { .. })
  )
}

fn is_divider(component: &ComponentValue) -> bool {
  match component {
    ComponentValue::PreservedToken(token) => match token.token {
      Token::Comma { .. } => true,
      Token::Delim { value } if value == '/' || value == ',' => true,
      _ => false,
    },
    _ => false,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::postcss::transform::{TransformContext, TransformCssOptions};
  use swc_core::common::{FileName, SourceMap, input::StringInput};
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
    .expect("failed to parse test stylesheet")
  }

  fn serialize_stylesheet(stylesheet: &Stylesheet) -> String {
    use swc_core::css::codegen::{
      CodeGenerator, CodegenConfig, Emit, writer::basic::BasicCssWriter,
    };

    let mut output = String::new();
    {
      let writer = BasicCssWriter::new(&mut output, None, Default::default());
      let mut generator = CodeGenerator::new(writer, CodegenConfig { minify: true });
      generator.emit(stylesheet).expect("emit stylesheet");
    }
    output
  }

  #[test]
  fn collapses_basic_whitespace() {
    let mut stylesheet = parse_stylesheet(".a { color : red ; }\n");
    let options = TransformCssOptions::default();
    let mut ctx = TransformContext::new(&options);
    NormalizeWhitespace.run(&mut stylesheet, &mut ctx);

    let output = serialize_stylesheet(&stylesheet);
    assert_eq!(output.trim_end(), ".a{color:red}");
  }

  #[test]
  fn trims_function_spacing() {
    let mut stylesheet = parse_stylesheet(".a { transform: translate( 10px , 20px ); }");
    let options = TransformCssOptions::default();
    let mut ctx = TransformContext::new(&options);
    NormalizeWhitespace.run(&mut stylesheet, &mut ctx);

    let output = serialize_stylesheet(&stylesheet);
    assert_eq!(output.trim_end(), ".a{transform:translate(10px,20px)}");
  }

  #[test]
  fn preserves_custom_property_empty_value() {
    let mut stylesheet = parse_stylesheet(".a { --foo: ; }");
    let options = TransformCssOptions::default();
    let mut ctx = TransformContext::new(&options);
    NormalizeWhitespace.run(&mut stylesheet, &mut ctx);

    let output = serialize_stylesheet(&stylesheet);
    assert_eq!(output.trim_end(), ".a{--foo: }");
  }
}
