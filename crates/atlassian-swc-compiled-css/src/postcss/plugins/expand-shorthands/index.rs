use swc_core::css::ast::{ComponentValue, Declaration, Rule, Stylesheet};

use super::super::super::transform::{Plugin, TransformContext};
use super::background::background;
use super::flex::flex;
use super::flex_flow::flex_flow;
use super::margin::margin;
use super::outline::outline;
use super::overflow::overflow;
use super::padding::padding;
use super::place_content::place_content;
use super::place_items::place_items;
use super::place_self::place_self;
use super::text_decoration::text_decoration;
use super::types::{
  LonghandDeclaration, ValuesRoot, clone_with_new_name, declaration_property_name,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct ExpandShorthands;

impl Plugin for ExpandShorthands {
  fn name(&self) -> &'static str {
    "expand-shorthands"
  }

  fn run(&self, stylesheet: &mut Stylesheet, _ctx: &mut TransformContext<'_>) {
    expand_rules(&mut stylesheet.rules);
  }
}

pub fn expand_shorthands() -> ExpandShorthands {
  ExpandShorthands
}

fn expand_rules(rules: &mut Vec<Rule>) {
  for rule in rules {
    match rule {
      Rule::QualifiedRule(rule) => expand_component_values(&mut rule.block.value),
      Rule::AtRule(at_rule) => {
        if let Some(block) = &mut at_rule.block {
          expand_component_values(&mut block.value);
        }
      }
      Rule::ListOfComponentValues(list) => expand_component_values(&mut list.children),
    }
  }
}

fn expand_component_values(values: &mut Vec<ComponentValue>) {
  let mut index = 0;
  while index < values.len() {
    let replacements = if let Some(ComponentValue::Declaration(declaration)) = values.get(index) {
      expand_declaration(declaration)
    } else {
      None
    };

    if let Some(replacements) = replacements {
      values.remove(index);
      if replacements.is_empty() {
        continue;
      }
      let replacement_count = replacements.len();
      for replacement in replacements.into_iter().rev() {
        values.insert(index, ComponentValue::Declaration(Box::new(replacement)));
      }
      index += replacement_count;
      continue;
    }

    if let Some(component) = values.get_mut(index) {
      match component {
        ComponentValue::SimpleBlock(block) => expand_component_values(&mut block.value),
        ComponentValue::Function(function) => expand_component_values(&mut function.value),
        ComponentValue::ListOfComponentValues(list) => expand_component_values(&mut list.children),
        ComponentValue::QualifiedRule(rule) => expand_component_values(&mut rule.block.value),
        ComponentValue::AtRule(at_rule) => {
          if let Some(block) = &mut at_rule.block {
            expand_component_values(&mut block.value);
          }
        }
        ComponentValue::KeyframeBlock(block) => expand_component_values(&mut block.block.value),
        _ => {}
      }
    }

    index += 1;
  }
}

pub fn expand_declaration(declaration: &Declaration) -> Option<Vec<Declaration>> {
  let property = declaration_property_name(&declaration.name);
  let converter = match conversion_for_property(&property) {
    Some(converter) => converter,
    None => {
      if std::env::var("COMPILED_CLI_TRACE").is_ok() {
        eprintln!(
          "[expand-shorthands:SWC] skip prop='{}' (no converter)",
          property
        );
      }
      return None;
    }
  };

  let values_root = ValuesRoot::from_components(&declaration.value);
  if values_root.is_empty() {
    if std::env::var("COMPILED_CLI_TRACE").is_ok() {
      eprintln!("[expand-shorthands:SWC] skip prop='{}' (empty)", property);
    }
    return None;
  }
  if values_root.contains_var_function() {
    if std::env::var("COMPILED_CLI_TRACE").is_ok() {
      eprintln!(
        "[expand-shorthands:SWC] skip prop='{}' (contains var())",
        property
      );
    }
    return None;
  }

  if std::env::var("COMPILED_CLI_TRACE").is_ok() {
    eprintln!("[expand-shorthands:SWC] visit prop='{}'", property);
  }

  let expanded = converter(&values_root);
  if expanded.is_empty() {
    if std::env::var("COMPILED_CLI_TRACE").is_ok() {
      eprintln!(
        "[expand-shorthands:SWC] expanded prop='{}' -> <empty>",
        property
      );
    }
    return Some(Vec::new());
  }

  if expanded.len() == 1 && matches!(expanded[0], LonghandDeclaration::KeepOriginal) {
    if std::env::var("COMPILED_CLI_TRACE").is_ok() {
      eprintln!("[expand-shorthands:SWC] keep original prop='{}'", property);
    }
    return None;
  }

  let mut declarations: Vec<Declaration> = Vec::new();

  for entry in expanded {
    match entry {
      LonghandDeclaration::Replace { prop, value } => {
        if std::env::var("COMPILED_CLI_TRACE").is_ok() {
          let text = super::types::serialize_component_values(&value).unwrap_or_default();
          eprintln!("[expand-shorthands:SWC]   -> {}:{}", prop, text);
        }
        declarations.push(clone_with_new_name(declaration, &prop, value));
      }
      LonghandDeclaration::KeepOriginal => {
        if std::env::var("COMPILED_CLI_TRACE").is_ok() {
          eprintln!(
            "[expand-shorthands:SWC] keep original prop='{}' (seen later)",
            property
          );
        }
        return None;
      }
    }
  }

  Some(declarations)
}

/// Engine helper: expand a (prop, value) pair into longhand (prop, value) string pairs
/// using the same conversion logic as the SWC plugin.
pub fn expand_shorthand_pairs(prop: &str, value: &str) -> Option<Vec<(String, String)>> {
  let converter = match conversion_for_property(prop) {
    Some(c) => c,
    None => return None,
  };
  let components = super::types::parse_value_to_components(value);
  let values_root = ValuesRoot::from_components(&components);
  if values_root.is_empty() || values_root.contains_var_function() {
    return None;
  }
  let expanded = converter(&values_root);
  if expanded.is_empty() {
    return Some(Vec::new());
  }
  if expanded.len() == 1 && matches!(expanded[0], LonghandDeclaration::KeepOriginal) {
    return None;
  }
  let mut out: Vec<(String, String)> = Vec::new();
  for entry in expanded {
    if let LonghandDeclaration::Replace { prop, value } = entry {
      let text = super::types::serialize_component_values(&value).unwrap_or_default();
      out.push((prop, text));
    } else {
      return None;
    }
  }
  Some(out)
}

fn conversion_for_property(property: &str) -> Option<fn(&ValuesRoot) -> Vec<LonghandDeclaration>> {
  match property {
    "background" => Some(background),
    "flex" => Some(flex),
    "flex-flow" => Some(flex_flow),
    "margin" => Some(margin),
    "outline" => Some(outline),
    "overflow" => Some(overflow),
    "padding" => Some(padding),
    "place-content" => Some(place_content),
    "place-items" => Some(place_items),
    "place-self" => Some(place_self),
    "text-decoration" => Some(text_decoration),
    _ => None,
  }
}
