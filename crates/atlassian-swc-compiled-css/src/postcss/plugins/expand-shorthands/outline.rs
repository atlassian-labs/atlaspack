use super::types::{parse_value_to_components, LonghandDeclaration, ValueNode, ValuesRoot};
use super::utils::{is_color, is_width, GLOBAL_VALUES};

const STYLE_VALUES: &[&str] = &[
  "auto", "none", "dotted", "dashed", "solid", "double", "groove", "ridge", "inset", "outset",
];
const SIZE_VALUES: &[&str] = &["thin", "medium", "thick"];

/// Expand the `outline` shorthand into color, style, and width longhands.
pub fn outline(value: &ValuesRoot) -> Vec<LonghandDeclaration> {
  let left = value.nodes.get(0);
  let middle = value.nodes.get(1);
  let right = value.nodes.get(2);

  let mut color_value: Option<ValueNode> = None;
  let mut style_value: Option<ValueNode> = None;
  let mut width_value: Option<ValueNode> = None;

  let mut extract = |node: Option<&ValueNode>| -> bool {
    if let Some(node) = node {
      if is_color(node) {
        if color_value.is_some() {
          return true;
        }
        color_value = Some(node.clone());
      } else if let Some(word) = node.as_word() {
        if SIZE_VALUES.contains(&word) {
          if width_value.is_some() {
            return true;
          }
          width_value = Some(node.clone());
        } else if is_valid_style(word) {
          if style_value.is_some() {
            return true;
          }
          style_value = Some(node.clone());
        } else {
          return true;
        }
      } else if is_numeric_outline_width(node) {
        if width_value.is_some() {
          return true;
        }
        width_value = Some(node.clone());
      }
    }
    false
  };

  if extract(left) || extract(middle) || extract(right) {
    return Vec::new();
  }

  let color_components = color_value
    .map(|node| node.clone_components())
    .unwrap_or_else(|| parse_value_to_components("currentColor"));
  let style_components = style_value
    .map(|node| node.clone_components())
    .unwrap_or_else(|| parse_value_to_components("none"));
  let width_components = width_value
    .map(|node| node.clone_components())
    .unwrap_or_else(|| parse_value_to_components("medium"));

  vec![
    LonghandDeclaration::replace("outline-color", color_components),
    LonghandDeclaration::replace("outline-style", style_components),
    LonghandDeclaration::replace("outline-width", width_components),
  ]
}

fn is_valid_style(word: &str) -> bool {
  GLOBAL_VALUES.contains(&word) || STYLE_VALUES.contains(&word)
}

fn is_numeric_outline_width(node: &ValueNode) -> bool {
  node.is_numeric_without_unit() || is_width(node)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::postcss::plugins::expand_shorthands::types::{
    parse_value_to_components, serialize_component_values,
  };

  fn to_pairs(result: &[LonghandDeclaration]) -> Vec<(String, String)> {
    result
      .iter()
      .map(|entry| match entry {
        LonghandDeclaration::Replace { prop, value } => {
          (prop.clone(), serialize_component_values(value).unwrap())
        }
        LonghandDeclaration::KeepOriginal => ("keep".into(), String::new()),
      })
      .collect()
  }

  #[test]
  fn expands_outline_components() {
    let root = ValuesRoot::from_components(&parse_value_to_components("red dotted 2px"));
    let result = outline(&root);
    assert_eq!(
      to_pairs(&result),
      vec![
        ("outline-color".into(), "red".into()),
        ("outline-style".into(), "dotted".into()),
        ("outline-width".into(), "2px".into()),
      ]
    );
  }

  #[test]
  fn falls_back_to_defaults() {
    let root = ValuesRoot::from_components(&parse_value_to_components(""));
    let result = outline(&root);
    assert_eq!(
      to_pairs(&result),
      vec![
        ("outline-color".into(), "currentColor".into()),
        ("outline-style".into(), "none".into()),
        ("outline-width".into(), "medium".into()),
      ]
    );
  }
}
