use crate::types::Bundle;

pub trait BundleGraph {
  // Temporary code just to validate functionality
  fn get_bundles(&self) -> Vec<&Bundle>;
}
