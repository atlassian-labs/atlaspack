use std::{ops::Deref, path::Path, sync::Arc};

use petgraph::{
  graph::NodeIndex,
  prelude::StableDiGraph,
  visit::{EdgeRef, IntoNodeReferences},
  Direction,
};

use crate::{
  as_variant_impl,
  asset_graph::{AssetNode, DependencyNode},
  types::{Asset, AssetId, Bundle, FileType, Priority},
};

/// We're hiding some of the asset on this value.
///
/// Ideally, we shouldn't be carrying around information on our graph structures, and instead centralize it on
/// a different piece of storage.
#[derive(Clone, PartialEq, Debug)]
pub struct AssetRef {
  asset_graph_node: AssetNode,
  asset_graph_node_index: NodeIndex,
}

impl AssetRef {
  pub fn new(asset_graph_node: AssetNode, asset_graph_node_index: NodeIndex) -> Self {
    Self {
      asset_graph_node,
      asset_graph_node_index,
    }
  }

  pub fn asset(&self) -> &Asset {
    &self.asset_graph_node.asset
  }

  pub fn file_path(&self) -> &Path {
    &self.asset_graph_node.asset.file_path
  }

  pub fn file_type(&self) -> &FileType {
    &self.asset_graph_node.asset.file_type
  }

  pub fn asset_graph_node_index(&self) -> NodeIndex {
    self.asset_graph_node_index
  }

  pub fn id(&self) -> AssetId {
    self.asset_graph_node.id()
  }
}

impl Deref for AssetRef {
  type Target = AssetNode;

  fn deref(&self) -> &Self::Target {
    &self.asset_graph_node
  }
}

#[derive(Debug, Clone)]
pub struct BundleGraphBundle {
  pub bundle: Bundle,
  // TODO: This should not be public
  pub assets: StableDiGraph<AssetRef, ()>,
}

impl std::fmt::Display for BundleGraphBundle {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "BundleGraphBundle(id={}, name={}, assets={})",
      self.bundle.id,
      self
        .bundle
        .name
        .as_ref()
        .unwrap_or(&"<unnamed>".to_string()),
      self.assets.node_count()
    )
  }
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum BundleGraphNode {
  Root,
  Entry,
  Bundle(Arc<BundleGraphBundle>),
}

as_variant_impl!(BundleGraphNode, as_bundle, Bundle, Arc<BundleGraphBundle>);

impl std::fmt::Display for BundleGraphNode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      BundleGraphNode::Root => write!(f, "Root"),
      BundleGraphNode::Entry => write!(f, "Entry"),
      BundleGraphNode::Bundle(bundle) => write!(f, "{}", bundle),
    }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BundleDependency {
  dependency_node: DependencyNode,
  target_assets: Vec<AssetRef>,
}

impl BundleDependency {
  pub fn new(dependency_node: &DependencyNode) -> Self {
    Self {
      dependency_node: dependency_node.clone(),
      target_assets: vec![],
    }
  }

  pub fn id(&self) -> String {
    self.dependency_node.id()
  }

  pub fn placeholder(&self) -> Option<&str> {
    self.dependency_node.dependency.placeholder.as_deref()
  }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BundleGraphEdge {
  /// Root to bundle, means the bundle is an entry-point
  RootEntryOf,
  /// Root to bundle, means the bundle is a shared bundle
  RootSharedBundleOf,
  /// Root to bundle, means the bundle is an async bundle
  RootAsyncBundleOf,
  /// Root to bundle, means the bundle is a type change bundle
  RootTypeChangeBundleOf,
  /// Bundle to bundle, means the bundle will be async loaded by the other
  BundleAsyncLoads(BundleDependency),
  /// Bundle to bundle, means the bundle will be sync loaded by the other
  BundleSyncLoads(BundleDependency),
}

#[derive(Debug, Clone)]
pub struct BundleGraph {
  graph: StableDiGraph<BundleGraphNode, BundleGraphEdge>,
  root: NodeIndex,
}

impl PartialEq for BundleGraph {
  fn eq(&self, _other: &Self) -> bool {
    false
  }
}

#[derive(Debug, Clone)]
pub struct BundleRef {
  bundle_graph_bundle: Arc<BundleGraphBundle>,
  bundle_node_index: NodeIndex,
}

impl BundleRef {
  pub fn bundle_graph_bundle(&self) -> &BundleGraphBundle {
    &self.bundle_graph_bundle
  }

  pub fn bundle_type(&self) -> &FileType {
    &self.bundle_graph_bundle.bundle.bundle_type
  }

  pub fn num_assets(&self) -> usize {
    self.bundle_graph_bundle.assets.node_count()
  }

  pub fn assets(&self) -> impl Iterator<Item = &AssetRef> + '_ {
    self.bundle_graph_bundle.assets.node_weights()
  }

  pub fn node_index(&self) -> NodeIndex {
    self.bundle_node_index
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundlePriority {
  Parallel,
  Lazy,
}

pub struct ReferencedBundle {
  bundle_ref: BundleRef,
  priority: BundlePriority,
}

impl ReferencedBundle {
  pub fn bundle_graph_bundle(&self) -> &BundleGraphBundle {
    &self.bundle_ref.bundle_graph_bundle
  }

  pub fn priority(&self) -> BundlePriority {
    self.priority
  }
}

impl Default for BundleGraph {
  fn default() -> Self {
    Self::new()
  }
}

impl BundleGraph {
  pub fn new() -> Self {
    let mut graph = StableDiGraph::new();
    let root = graph.add_node(BundleGraphNode::Root);

    Self { graph, root }
  }

  pub fn build_from(
    root: NodeIndex,
    graph: StableDiGraph<BundleGraphNode, BundleGraphEdge>,
  ) -> Self {
    Self { graph, root }
  }

  pub fn root(&self) -> NodeIndex {
    self.root
  }

  pub fn graph(&self) -> &StableDiGraph<BundleGraphNode, BundleGraphEdge> {
    &self.graph
  }

  pub fn bundles(&self) -> impl Iterator<Item = BundleRef> + '_ {
    self
      .graph
      .node_references()
      .filter_map(|(node_index, weight)| match weight {
        BundleGraphNode::Bundle(bundle) => Some(BundleRef {
          bundle_graph_bundle: bundle.clone(),
          bundle_node_index: node_index,
        }),
        _ => None,
      })
  }

  pub fn num_bundles(&self) -> usize {
    self
      .graph
      .node_weights()
      .filter(|weight| matches!(weight, BundleGraphNode::Bundle(_)))
      .count()
  }

  pub fn add_bundle(&mut self, edge: BundleGraphEdge, bundle: BundleGraphBundle) -> NodeIndex {
    let node_index = self
      .graph
      .add_node(BundleGraphNode::Bundle(Arc::new(bundle)));

    self.add_edge(self.root(), node_index, edge);

    node_index
  }

  pub fn add_edge(&mut self, source: NodeIndex, target: NodeIndex, weight: BundleGraphEdge) {
    self.graph.add_edge(source, target, weight);
  }

  pub fn referenced_bundles(&self, bundle: &BundleRef) -> Vec<ReferencedBundle> {
    let bundle_node_index = bundle.bundle_node_index;
    let edges = self
      .graph
      .edges_directed(bundle_node_index, Direction::Outgoing);

    let mut referenced_bundles = Vec::new();
    for edge in edges {
      match edge.weight() {
        BundleGraphEdge::BundleSyncLoads(_) | BundleGraphEdge::BundleAsyncLoads(_) => {
          let node_index = edge.target();
          let node = self.graph.node_weight(node_index).unwrap();
          let bundle = node
            .as_bundle()
            .expect("Sync loads relation onto a non-bundle node");

          referenced_bundles.push(ReferencedBundle {
            bundle_ref: BundleRef {
              bundle_graph_bundle: bundle.clone(),
              bundle_node_index: node_index,
            },
            priority: if matches!(edge.weight(), BundleGraphEdge::BundleSyncLoads(_)) {
              BundlePriority::Parallel
            } else {
              BundlePriority::Lazy
            },
          });
        }
        _ => {}
      }
    }

    referenced_bundles
  }
}
