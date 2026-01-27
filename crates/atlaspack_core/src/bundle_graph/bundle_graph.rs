use crate::types::{Asset, Bundle};

pub trait BundleGraph {
  fn get_bundle_by_id(&self, id: &str) -> Option<&Bundle>;
  fn traverse_bundle_assets(&self, bundle: &Bundle, visit: impl FnMut(&Asset));
}
