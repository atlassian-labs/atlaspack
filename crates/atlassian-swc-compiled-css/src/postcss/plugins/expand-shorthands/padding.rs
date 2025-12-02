use super::types::{LonghandDeclaration, ValuesRoot};

/// Expand the `padding` shorthand into its longhand properties.
pub fn padding(value: &ValuesRoot) -> Vec<LonghandDeclaration> {
  if value.nodes.is_empty() {
    return vec![LonghandDeclaration::keep_original()];
  }

  let top = &value.nodes[0];
  let right = value.nodes.get(1).unwrap_or(top);
  let bottom = value.nodes.get(2).unwrap_or(top);
  let left = value.nodes.get(3).unwrap_or(right);

  vec![
    LonghandDeclaration::replace("padding-top", top.clone_components()),
    LonghandDeclaration::replace("padding-right", right.clone_components()),
    LonghandDeclaration::replace("padding-bottom", bottom.clone_components()),
    LonghandDeclaration::replace("padding-left", left.clone_components()),
  ]
}
