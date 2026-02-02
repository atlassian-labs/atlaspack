pub mod bundle_graph;
pub mod bundle_graph_from_js;
pub mod native_bundle_graph;

// Temporary trait used by JS->Rust bundle graph loading code.
// This will be removed once native bundling fully replaces the JS path.
pub trait BundleGraphTrait {
  fn get_bundles(&self) -> Vec<&crate::types::Bundle>;

  fn get_bundle_assets(
    &self,
    bundle: &crate::types::Bundle,
  ) -> anyhow::Result<Vec<&crate::types::Asset>>;
}

pub use bundle_graph::*;
pub use native_bundle_graph::NativeBundleGraph;
