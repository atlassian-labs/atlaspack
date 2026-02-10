use atlaspack_core::{asset_graph::AssetGraph, bundle_graph::NativeBundleGraph};

pub mod ideal_graph;
pub mod monolithic;

pub use ideal_graph::IdealGraphBundler;
pub use monolithic::MonolithicBundler;

/// Bundler algorithms take an asset graph and assign assets/dependencies to bundles.
///
/// Implementations are expected to mutate the provided `NativeBundleGraph` to:
/// - create bundle / bundle_group nodes (`bundle_nodes`)
/// - create bundle membership edges (`bundle_edges`)
pub trait Bundler {
  fn bundle(
    &self,
    asset_graph: &AssetGraph,
    bundle_graph: &mut NativeBundleGraph,
  ) -> anyhow::Result<()>;
}
