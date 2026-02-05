//! "Ideal graph" bundling algorithm scaffolding.
//!
//! This module is an initial landing zone for the algorithm described in
//! `bundler-rust-rewrite-research.md` ("ideal graph").
//!
//! Goals of this scaffolding:
//! - Provide stable, testable Rust types to iterate on.
//! - Keep algorithm phases explicit (build graph, compute boundaries, dominators, placement, etc.).
//! - Avoid coupling to Parcel/JS implementation details so we can evolve safely.

pub mod types;

use anyhow::Context;
use atlaspack_core::{
  asset_graph::AssetGraph,
  bundle_graph::{NativeBundleGraph, native_bundle_graph::NativeBundleGraphEdgeType},
};

use crate::Bundler;

use self::types::{IdealGraph, IdealGraphBuildOptions, IdealGraphBuildStats};

/// Bundler implementation backed by the (future) ideal graph algorithm.
///
/// For now this is a no-op scaffolding that builds an [`IdealGraph`] from the asset graph
/// and emits a minimal set of invariants into the provided [`NativeBundleGraph`].
#[derive(Debug, Default)]
pub struct IdealGraphBundler {
  pub options: IdealGraphBuildOptions,
}

impl IdealGraphBundler {
  pub fn new(options: IdealGraphBuildOptions) -> Self {
    Self { options }
  }

  /// Builds the intermediate ideal graph representation.
  pub fn build_ideal_graph(
    &self,
    asset_graph: &AssetGraph,
  ) -> anyhow::Result<(IdealGraph, IdealGraphBuildStats)> {
    IdealGraph::from_asset_graph(asset_graph, &self.options)
      .context("building IdealGraph from AssetGraph")
  }
}

impl Bundler for IdealGraphBundler {
  fn bundle(
    &self,
    asset_graph: &AssetGraph,
    bundle_graph: &mut NativeBundleGraph,
  ) -> anyhow::Result<()> {
    let (_ideal_graph, _stats) = self.build_ideal_graph(asset_graph)?;

    // Placeholder: until placement is implemented, keep bundle graph untouched.
    // We *do* assert we can at least access the root node, so we fail loudly if
    // the graph construction changes underneath us.
    let root_node_id = *bundle_graph
      .get_node_id_by_content_key("@@root")
      .context("missing @@root node in NativeBundleGraph")?;

    // No-op edge to ensure the method touches the bundle graph in a deterministic way.
    // This is intentionally `Null` to avoid affecting semantics.
    bundle_graph.add_edge(
      &root_node_id,
      &root_node_id,
      NativeBundleGraphEdgeType::Null,
    );

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use atlaspack_core::{
    asset_graph::AssetGraph,
    bundle_graph::NativeBundleGraph,
    types::{Asset, Dependency, Environment, FileType, Target},
  };

  use super::*;

  #[test]
  fn ideal_graph_bundler_can_build_graph() {
    let mut asset_graph = AssetGraph::new();

    let target = Target::default();
    let entry_dep = Dependency::entry("entry.js".to_string(), target);
    let entry_dep_node = asset_graph.add_entry_dependency(entry_dep, false);

    let entry_asset = Arc::new(Asset {
      // Use a hex-like id so NativeBundleGraph public_id generation doesn't panic.
      id: "deadbeefdeadbeef".into(),
      file_path: "entry.js".into(),
      file_type: FileType::Js,
      env: Arc::new(Environment::default()),
      ..Asset::default()
    });
    let entry_asset_node = asset_graph.add_asset(entry_asset, false);
    asset_graph.add_edge(&entry_dep_node, &entry_asset_node);

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions::default());
    let (g, stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_eq!(stats.assets, 1);
    assert_eq!(stats.dependencies, 1);
    assert!(!g.nodes.is_empty());

    // Ensure the `Bundler` impl is wired and can be called.
    let mut bundle_graph = NativeBundleGraph::from_asset_graph(&asset_graph);
    bundler.bundle(&asset_graph, &mut bundle_graph).unwrap();

    // Avoid unused warnings.
    let _ = entry_asset_node;
  }
}
