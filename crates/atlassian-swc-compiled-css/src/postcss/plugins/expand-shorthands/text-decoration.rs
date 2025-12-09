use super::types::{LonghandDeclaration, ValuesRoot, parse_value_to_components};
use super::utils::{GLOBAL_VALUES, is_color};

const STYLE_VALUES: &[&str] = &["solid", "double", "dotted", "dashed", "wavy"];
const LINE_VALUES: &[&str] = &["none", "underline", "overline", "line-through", "blink"];

/// Expand the `text-decoration` shorthand.
pub fn text_decoration(value: &ValuesRoot) -> Vec<LonghandDeclaration> {
  let left = value.nodes.get(0);
  let middle = value.nodes.get(1);
  let right = value.nodes.get(2);

  let mut color: Option<String> = None;
  let mut style: Option<String> = None;
  let mut line_values: Vec<String> = Vec::new();

  let mut extract = |node: Option<&super::types::ValueNode>| -> bool {
    if let Some(node) = node {
      if let Some(word) = node.as_word() {
        if is_line_value(word) {
          if line_values.iter().any(|existing| existing == word) {
            return true;
          }
          line_values.push(word.to_string());
        } else if is_color(node) {
          color = Some(word.to_string());
        } else if is_style_value(word) {
          style = Some(word.to_string());
        }
      }
    }
    false
  };

  if extract(left) || extract(middle) || extract(right) {
    return Vec::new();
  }

  line_values.sort();
  let line_value = if line_values.is_empty() {
    "none".to_string()
  } else {
    line_values.join(" ")
  };

  let color_components = color
    .map(|value| parse_value_to_components(&value))
    .unwrap_or_else(|| parse_value_to_components("currentColor"));
  let style_components = style
    .map(|value| parse_value_to_components(&value))
    .unwrap_or_else(|| parse_value_to_components("solid"));
  let line_components = parse_value_to_components(&line_value);

  vec![
    LonghandDeclaration::replace("text-decoration-color", color_components),
    LonghandDeclaration::replace("text-decoration-line", line_components),
    LonghandDeclaration::replace("text-decoration-style", style_components),
  ]
}

fn is_line_value(word: &str) -> bool {
  GLOBAL_VALUES.contains(&word) || LINE_VALUES.contains(&word)
}

fn is_style_value(word: &str) -> bool {
  GLOBAL_VALUES.contains(&word) || STYLE_VALUES.contains(&word)
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
  fn sorts_line_values() {
    let root = ValuesRoot::from_components(&parse_value_to_components("overline underline red"));
    let result = text_decoration(&root);
    assert_eq!(
      to_pairs(&result),
      vec![
        ("text-decoration-color".into(), "red".into()),
        ("text-decoration-line".into(), "overline underline".into()),
        ("text-decoration-style".into(), "solid".into()),
      ]
    );
  }

  #[test]
  fn rejects_duplicate_lines() {
    let root = ValuesRoot::from_components(&parse_value_to_components("underline underline"));
    let result = text_decoration(&root);
    assert!(result.is_empty());
  }
}
