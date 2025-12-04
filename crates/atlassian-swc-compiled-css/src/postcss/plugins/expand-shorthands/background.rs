use super::types::{LonghandDeclaration, ValuesRoot};
use super::utils::is_color;

/// Only the `background-color` property is expanded. All other values remain untouched.
pub fn background(value: &ValuesRoot) -> Vec<LonghandDeclaration> {
  if value.nodes.len() == 1 {
    let node = &value.nodes[0];
    if is_color(node) {
      return vec![LonghandDeclaration::replace(
        "background-color",
        node.clone_components(),
      )];
    }
  }

  vec![LonghandDeclaration::keep_original()]
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::postcss::plugins::expand_shorthands::types::{
    parse_value_to_components, serialize_component_values,
  };

  #[test]
  fn expands_single_color() {
    let root = ValuesRoot::from_components(&parse_value_to_components("#fff"));
    let result = background(&root);
    assert_eq!(result.len(), 1);
    match &result[0] {
      LonghandDeclaration::Replace { prop, value } => {
        assert_eq!(prop, "background-color");
        assert_eq!(serialize_component_values(value).unwrap(), "#fff");
      }
      _ => panic!("expected replacement"),
    }
  }

  #[test]
  fn keeps_original_for_non_color() {
    let root = ValuesRoot::from_components(&parse_value_to_components("url(image.png)"));
    let result = background(&root);
    assert!(matches!(
      result.as_slice(),
      [LonghandDeclaration::KeepOriginal]
    ));
  }
}
