use petgraph::prelude::StableDiGraph;

use crate::types::{AssetId, Bundle, Environment, FileType, Target};

#[derive(Debug, Clone, PartialEq)]
pub struct BundleGraphBundle {
  pub bundle: Bundle,
  // TODO: This should not be public
  pub assets: Vec<AssetId>,
}

impl BundleGraphBundle {
  pub fn empty() -> Self {
    Self {
      bundle: Bundle {
        bundle_behavior: None,
        bundle_type: FileType::Js,
        entry_asset_ids: vec![],
        env: Environment::default(),
        hash_reference: "".to_string(),
        id: "".to_string(),
        is_splittable: true,
        main_entry_id: None,
        manual_shared_bundle: None,
        name: None,
        needs_stable_name: false,
        pipeline: None,
        public_id: None,
        target: Target::default(),
      },
      assets: vec![],
    }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BundleGraphNode {
  Root,
  Entry,
  Bundle(BundleGraphBundle),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BundleGraphEdge {
  AsyncLoads,
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
