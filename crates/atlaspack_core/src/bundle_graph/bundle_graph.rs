use crate::types::{Asset, Bundle, Dependency};

pub trait BundleGraph {
  fn get_bundle_assets(&self, bundle: &Bundle) -> anyhow::Result<Vec<&Asset>>;

  fn get_bundle_by_id(&self, id: &str) -> Option<&Bundle>;

  /// Get the public ID for an asset by its full asset ID.
  ///
  /// Public IDs are shortened, base62-encoded versions of asset IDs used at runtime.
  fn get_public_asset_id(&self, asset_id: &str) -> Option<&str>;

  /// Returns all of the Dependencies for an Asset (the dependencies that the asset requires)
  fn get_dependencies(&self, asset: &Asset) -> anyhow::Result<Vec<&Dependency>>;

  /// Resolves a dependency to an asset in a bundle
  fn get_resolved_asset(
    &self,
    dependency: &Dependency,
    bundle: &Bundle,
  ) -> anyhow::Result<Option<&Asset>>;

  /// Returns whether a dependency was excluded because it had no used symbols.
  fn is_dependency_skipped(&self, dependency: &Dependency) -> bool;
}
