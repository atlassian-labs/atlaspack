use crate::types::{Asset, Bundle};

pub trait BundleGraph {
  fn get_bundle_by_id(&self, id: &str) -> Option<&Bundle>;
  fn traverse_bundle_assets(&self, bundle: &Bundle, visit: impl FnMut(&Asset));
  /// Get the public ID for an asset by its full asset ID.
  ///
  /// Public IDs are shortened, base62-encoded versions of asset IDs used at runtime.
  fn get_public_asset_id(&self, asset_id: &str) -> Option<&str>;
}
