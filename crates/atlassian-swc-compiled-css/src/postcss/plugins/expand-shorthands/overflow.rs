use super::types::{LonghandDeclaration, ValuesRoot};

/// Expand the `overflow` shorthand into `overflow-x` and `overflow-y`.
pub fn overflow(value: &ValuesRoot) -> Vec<LonghandDeclaration> {
  if value.nodes.is_empty() {
    return vec![LonghandDeclaration::keep_original()];
  }

  let overflow_x = &value.nodes[0];
  let overflow_y = value.nodes.get(1).unwrap_or(overflow_x);

  vec![
    LonghandDeclaration::replace("overflow-x", overflow_x.clone_components()),
    LonghandDeclaration::replace("overflow-y", overflow_y.clone_components()),
  ]
}
