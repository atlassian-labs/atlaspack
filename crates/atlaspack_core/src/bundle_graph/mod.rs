use petgraph::prelude::StableDiGraph;

use crate::types::{AssetId, Bundle};

#[derive(Debug, Clone)]
pub struct BundleGraphBundle {
  pub bundle: Bundle,
  // TODO: This should not be public
  pub assets: StableDiGraph<u64, ()>,
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum BundleGraphNode {
  Root,
  Entry,
  Bundle(BundleGraphBundle),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BundleGraphEdge {
  /// Root to bundle, means the bundle is an entry-point
  EntryOf,
  /// Root to bundle, means the bundle is a shared bundle
  SharedBundleOf,
  /// Root to bundle, means the bundle is an async bundle
  AsyncBundleOf,
  /// Bundle to bundle, means the bundle will be async loaded by the other
  AsyncLoads,
  /// Bundle to bundle, means the bundle will be sync loaded by the other
  SyncLoads,
}

#[derive(Debug, Clone, Default)]
pub struct BundleGraph {
  graph: StableDiGraph<BundleGraphNode, BundleGraphEdge>,
}

impl PartialEq for BundleGraph {
  fn eq(&self, _other: &Self) -> bool {
    false
  }
}

impl BundleGraph {
  pub fn new(graph: StableDiGraph<BundleGraphNode, BundleGraphEdge>) -> Self {
    Self { graph }
  }

  pub fn graph(&self) -> &StableDiGraph<BundleGraphNode, BundleGraphEdge> {
    &self.graph
  }

  pub fn num_bundles(&self) -> usize {
    self
      .graph
      .node_weights()
      .filter(|weight| matches!(weight, BundleGraphNode::Bundle(_)))
      .count()
  }

  pub fn add_bundle(&mut self, bundle: BundleGraphBundle) {
    self.graph.add_node(BundleGraphNode::Bundle(bundle));
  }
}
