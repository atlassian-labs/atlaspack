use super::types::{LonghandDeclaration, ValuesRoot};

/// Expand the `place-content` shorthand into `align-content` and `justify-content`.
pub fn place_content(value: &ValuesRoot) -> Vec<LonghandDeclaration> {
  if value.nodes.is_empty() {
    return vec![LonghandDeclaration::keep_original()];
  }

  let align_content = &value.nodes[0];
  if value.nodes.len() == 1 {
    if let Some(word) = align_content.as_word() {
      if matches!(word, "left" | "right" | "baseline") {
        return Vec::new();
      }
    }
  }

  let justify_content = value.nodes.get(1).unwrap_or(align_content);

  vec![
    LonghandDeclaration::replace("align-content", align_content.clone_components()),
    LonghandDeclaration::replace("justify-content", justify_content.clone_components()),
  ]
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::postcss::plugins::expand_shorthands::types::parse_value_to_components;

  #[test]
  fn rejects_invalid_single_value() {
    let root = ValuesRoot::from_components(&parse_value_to_components("left"));
    let result = place_content(&root);
    assert!(result.is_empty());
  }
}
