use crate::types::{Asset, Bundle};

pub trait BundleGraph {
  // Temporary code just to validate functionality
  fn get_bundles(&self) -> Vec<&Bundle>;

  fn traverse_bundle_assets(&self, bundle: &Bundle, start_asset: Option<&Asset>) -> Vec<&Asset>;
}
