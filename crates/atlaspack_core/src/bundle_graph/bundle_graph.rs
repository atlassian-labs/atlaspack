use crate::types::{Asset, Bundle};

pub trait BundleGraph {
  // Temporary code just to validate functionality
  fn get_bundles(&self) -> Vec<&Bundle>;

  fn get_bundle_assets(&self, bundle: &Bundle) -> anyhow::Result<Vec<&Asset>>;
}
