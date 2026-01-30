use super::types::{LonghandDeclaration, ValuesRoot};

/// Expand the `place-items` shorthand into `align-items` and `justify-items`.
pub fn place_items(value: &ValuesRoot) -> Vec<LonghandDeclaration> {
  if value.nodes.is_empty() {
    return vec![LonghandDeclaration::keep_original()];
  }

  let align_items = &value.nodes[0];
  let justify_items = value.nodes.get(1).unwrap_or(align_items);

  vec![
    LonghandDeclaration::replace("align-items", align_items.clone_components()),
    LonghandDeclaration::replace("justify-items", justify_items.clone_components()),
  ]
}
