use super::types::{LonghandDeclaration, ValueNode, ValuesRoot, parse_value_to_components};
use super::utils::GLOBAL_VALUES;

/// Expand the `flex-flow` shorthand into `flex-direction` and `flex-wrap`.
pub fn flex_flow(value: &ValuesRoot) -> Vec<LonghandDeclaration> {
  let left = value.nodes.get(0);
  let right = value.nodes.get(1);

  let mut direction_value: Option<ValueNode> = None;
  let mut wrap_value: Option<ValueNode> = None;

  let mut extract = |node: Option<&ValueNode>| -> bool {
    if let Some(node) = node {
      if let Some(word) = node.as_word() {
        if is_valid_direction(word) {
          if direction_value.is_some() {
            return true;
          }
          direction_value = Some(node.clone());
        } else if is_valid_wrap(word) {
          if wrap_value.is_some() {
            return true;
          }
          wrap_value = Some(node.clone());
        } else {
          return true;
        }
      }
    }
    false
  };

  if extract(left) || extract(right) {
    return Vec::new();
  }

  let direction_components = direction_value
    .map(|node| node.clone_components())
    .unwrap_or_else(|| parse_value_to_components("row"));
  let wrap_components = wrap_value
    .map(|node| node.clone_components())
    .unwrap_or_else(|| parse_value_to_components("nowrap"));

  vec![
    LonghandDeclaration::replace("flex-direction", direction_components),
    LonghandDeclaration::replace("flex-wrap", wrap_components),
  ]
}

fn is_valid_direction(word: &str) -> bool {
  GLOBAL_VALUES.contains(&word)
    || matches!(word, "row" | "row-reverse" | "column" | "column-reverse")
}

fn is_valid_wrap(word: &str) -> bool {
  GLOBAL_VALUES.contains(&word) || matches!(word, "nowrap" | "wrap" | "reverse")
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::postcss::plugins::expand_shorthands::types::{
    parse_value_to_components, serialize_component_values,
  };

  #[test]
  fn expands_defaults_when_missing() {
    let root = ValuesRoot::from_components(&parse_value_to_components(""));
    let result = flex_flow(&root);
    let outputs: Vec<_> = result
      .iter()
      .map(|entry| match entry {
        LonghandDeclaration::Replace { prop, value } => {
          (prop.clone(), serialize_component_values(value).unwrap())
        }
        LonghandDeclaration::KeepOriginal => ("keep".into(), String::new()),
      })
      .collect();
    assert_eq!(
      outputs,
      vec![
        ("flex-direction".into(), "row".into()),
        ("flex-wrap".into(), "nowrap".into()),
      ]
    );
  }

  #[test]
  fn rejects_invalid_tokens() {
    let root = ValuesRoot::from_components(&parse_value_to_components("foo"));
    let result = flex_flow(&root);
    assert!(result.is_empty());
  }
}
