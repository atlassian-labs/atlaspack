use crate::postcss::plugins::expand_shorthands::types::parse_value_to_components;
use crate::postcss::transform::{Plugin, TransformContext};
use swc_core::css::ast::{ComponentValue, Declaration, Rule, Stylesheet, Token};

#[derive(Debug, Default, Clone, Copy)]
pub struct ColorMinLite;

pub fn colormin_lite() -> ColorMinLite {
  ColorMinLite
}

impl Plugin for ColorMinLite {
  fn name(&self) -> &'static str {
    "postcss-colormin-lite"
  }

  fn run(&self, stylesheet: &mut Stylesheet, _ctx: &mut TransformContext<'_>) {
    for rule in &mut stylesheet.rules {
      minimize_rule(rule);
    }
  }
}

fn minimize_rule(rule: &mut Rule) {
  match rule {
    Rule::QualifiedRule(rule) => minimize_values(&mut rule.block.value),
    Rule::AtRule(at) => {
      if let Some(block) = &mut at.block {
        minimize_values(&mut block.value);
      }
    }
    Rule::ListOfComponentValues(list) => minimize_components(&mut list.children),
  }
}

fn minimize_components(values: &mut [ComponentValue]) {
  for v in values {
    match v {
      ComponentValue::Declaration(decl) => minimize_declaration(decl),
      ComponentValue::QualifiedRule(rule) => minimize_values(&mut rule.block.value),
      ComponentValue::AtRule(at) => {
        if let Some(block) = &mut at.block {
          minimize_values(&mut block.value);
        }
      }
      ComponentValue::SimpleBlock(block) => minimize_values(&mut block.value),
      ComponentValue::ListOfComponentValues(list) => minimize_components(&mut list.children),
      ComponentValue::Function(fun) => minimize_components(&mut fun.value),
      _ => {}
    }
  }
}

fn minimize_values(values: &mut Vec<ComponentValue>) {
  for v in values.iter_mut() {
    match v {
      ComponentValue::Declaration(decl) => minimize_declaration(decl),
      ComponentValue::QualifiedRule(rule) => minimize_values(&mut rule.block.value),
      ComponentValue::AtRule(at) => {
        if let Some(block) = &mut at.block {
          minimize_values(&mut block.value);
        }
      }
      ComponentValue::SimpleBlock(block) => minimize_values(&mut block.value),
      ComponentValue::ListOfComponentValues(list) => minimize_components(&mut list.children),
      ComponentValue::Function(fun) => minimize_components(&mut fun.value),
      _ => {}
    }
  }
}

fn minimize_declaration(decl: &mut Declaration) {
  if decl.value.len() != 1 {
    return;
  }
  // Handle both preserved token and direct ident
  let mut name_opt: Option<String> = None;
  match &decl.value[0] {
    ComponentValue::PreservedToken(tok) => {
      if let Token::Ident { value, .. } = &tok.token {
        name_opt = Some(value.as_ref().to_string());
      }
    }
    ComponentValue::Ident(ident) => {
      name_opt = Some(ident.value.as_ref().to_string());
    }
    _ => {}
  }
  if let Some(name) = name_opt {
    let replacement = match name.to_ascii_lowercase().as_str() {
      "black" => Some("#000"),
      "dodgerblue" => Some("#1e90ff"),
      _ => None,
    };
    if let Some(hex) = replacement {
      if hex.len() < name.len() {
        // Parse hex into proper CSS components (Hash token), avoiding escaped '#'
        decl.value = parse_value_to_components(hex);
      }
    }
  }
}
