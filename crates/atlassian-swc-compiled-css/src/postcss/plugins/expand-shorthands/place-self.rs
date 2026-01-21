use super::types::{LonghandDeclaration, ValuesRoot};

/// Expand the `place-self` shorthand into `align-self` and `justify-self`.
pub fn place_self(value: &ValuesRoot) -> Vec<LonghandDeclaration> {
  if value.nodes.is_empty() {
    return vec![LonghandDeclaration::keep_original()];
  }

  let align_self = &value.nodes[0];
  let justify_self = value.nodes.get(1).unwrap_or(align_self);

  vec![
    LonghandDeclaration::replace("align-self", align_self.clone_components()),
    LonghandDeclaration::replace("justify-self", justify_self.clone_components()),
  ]
}
