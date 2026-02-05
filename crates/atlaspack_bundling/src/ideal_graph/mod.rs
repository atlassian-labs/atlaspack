//! "Ideal graph" bundling algorithm scaffolding.
//!
//! This module is an initial landing zone for the algorithm described in
//! `bundler-rust-rewrite-research.md` ("ideal graph").
//!
//! Goals of this scaffolding:
//! - Provide stable, testable Rust types to iterate on.
//! - Keep algorithm phases explicit (build graph, compute boundaries, dominators, placement, etc.).
//! - Avoid coupling to Parcel/JS implementation details so we can evolve safely.

pub mod builder;
pub mod types;

use anyhow::Context;
use atlaspack_core::{
  asset_graph::AssetGraph,
  bundle_graph::{NativeBundleGraph, native_bundle_graph::NativeBundleGraphEdgeType},
};

use crate::Bundler;

use self::{
  builder::IdealGraphBuilder,
  types::{IdealGraph, IdealGraphBuildOptions, IdealGraphBuildStats},
};

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
    IdealGraphBuilder::new(self.options.clone())
      .build(asset_graph)
      .context("building IdealGraph via IdealGraphBuilder")
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

    // Add a lazy dep creating an async boundary.
    let lazy_dep = atlaspack_core::types::DependencyBuilder::default()
      .specifier("./async.js".to_string())
      .specifier_type(atlaspack_core::types::SpecifierType::Esm)
      .env(Arc::new(Environment::default()))
      .priority(atlaspack_core::types::Priority::Lazy)
      .source_asset_id("deadbeefdeadbeef".into())
      .build();
    let lazy_dep_node = asset_graph.add_dependency(lazy_dep, false);
    asset_graph.add_edge(&entry_asset_node, &lazy_dep_node);

    let async_asset = Arc::new(Asset {
      id: "async_asset".into(),
      file_path: "async.js".into(),
      file_type: FileType::Js,
      env: Arc::new(Environment::default()),
      ..Asset::default()
    });
    let async_asset_node = asset_graph.add_asset(async_asset, false);
    asset_graph.add_edge(&lazy_dep_node, &async_asset_node);

    let bundler = IdealGraphBundler::new(IdealGraphBuildOptions {
      collect_debug: true,
    });
    let (g, stats) = bundler.build_ideal_graph(&asset_graph).unwrap();

    assert_eq!(stats.assets, 2);
    assert_eq!(stats.dependencies, 2);

    // Both entry and boundary roots become bundles.
    assert_eq!(g.bundles.len(), 2);

    // Debug decision log should exist and contain boundary + placement decisions.
    let debug = g.debug.as_ref().expect("debug info should be present");
    assert!(!debug.decisions.is_empty());
    assert!(
      debug
        .decisions
        .decisions
        .iter()
        .any(|d| { matches!(d.kind, types::DecisionKind::BoundaryCreated { .. }) })
    );
    assert!(debug.decisions.decisions.iter().any(|d| {
      matches!(
        d.kind,
        types::DecisionKind::AssetAssignedToBundle { .. }
          | types::DecisionKind::BundleRootCreated { .. }
      )
    }));

    // Decisions are sequential.
    for (i, d) in debug.decisions.decisions.iter().enumerate() {
      assert_eq!(d.seq, i as u64);
    }

    // We should have a lazy bundle edge from entry bundle to the async bundle.
    assert!(g.bundle_edges.iter().any(|(from, to, ty)| {
      from.0 == "deadbeefdeadbeef"
        && to.0 == "async_asset"
        && matches!(ty, types::IdealEdgeType::Lazy)
    }));

    // NOTE: We intentionally do not call `NativeBundleGraph::from_asset_graph` here.
    // That path has stricter invariants about asset ids/public ids that aren't relevant
    // for unit testing the ideal graph pipeline.

    // Avoid unused warnings.
    let _ = (entry_asset_node, async_asset_node);
  }
}
