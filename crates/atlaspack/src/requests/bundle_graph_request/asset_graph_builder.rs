use std::path::PathBuf;

use atlaspack_core::{
  asset_graph::{AssetNode, DependencyNode},
  bundle_graph::AssetRef,
  types::{Asset, AssetId, FileType},
};
use petgraph::graph::NodeIndex;

use crate::requests::bundle_graph_request::{
  simplified_graph::SimplifiedAssetDependency, SimplifiedAssetGraph, SimplifiedAssetGraphEdge,
  SimplifiedAssetGraphNode,
};

pub struct SimplifiedAssetGraphBuilder {
  graph: SimplifiedAssetGraph,
  root: NodeIndex,
}

impl SimplifiedAssetGraphBuilder {
  pub fn root(&self) -> NodeIndex {
    self.root
  }

  pub fn entry_asset(&mut self, path: &str) -> NodeIndex {
    let asset = self
      .graph
      .add_node(SimplifiedAssetGraphNode::Asset(AssetRef::new(
        AssetNode::from(Asset {
          file_path: PathBuf::from(path),
          ..Asset::default()
        }),
        NodeIndex::new(self.graph.node_count()),
      )));

    self.graph.add_edge(
      self.root,
      asset,
      SimplifiedAssetGraphEdge::EntryAssetRoot(DependencyNode::default()),
    );

    asset
  }

  pub fn asset(&mut self, path: &str) -> NodeIndex {
    self.asset_with_type(path, FileType::default())
  }

  pub fn asset_with_type(&mut self, path: &str, file_type: FileType) -> NodeIndex {
    let asset = self
      .graph
      .add_node(SimplifiedAssetGraphNode::Asset(AssetRef::new(
        AssetNode::from(Asset {
          file_type,
          file_path: PathBuf::from(path),
          id: AssetId::from(self.graph.node_count() as u64),
          ..Asset::default()
        }),
        NodeIndex::new(self.graph.node_count()),
      )));

    asset
  }

  pub fn sync_dependency(&mut self, source: NodeIndex, target: NodeIndex) {
    self.graph.add_edge(
      source,
      target,
      SimplifiedAssetGraphEdge::AssetDependency(SimplifiedAssetDependency::new(
        DependencyNode::default(),
        target,
      )),
    );
  }

  pub fn async_dependency(&mut self, source: NodeIndex, target: NodeIndex) {
    self.graph.add_edge(
      source,
      target,
      SimplifiedAssetGraphEdge::AssetAsyncDependency(SimplifiedAssetDependency::new(
        DependencyNode::default(),
        target,
      )),
    );
    self.graph.add_edge(
      self.root,
      target,
      SimplifiedAssetGraphEdge::AsyncRoot(DependencyNode::default()),
    );
  }

  pub fn build(self) -> SimplifiedAssetGraph {
    self.graph
  }
}

pub fn simplified_asset_graph_builder() -> SimplifiedAssetGraphBuilder {
  let mut graph = SimplifiedAssetGraph::new();
  let root = graph.add_node(SimplifiedAssetGraphNode::Root);
  SimplifiedAssetGraphBuilder { graph, root }
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use petgraph::graph::NodeIndex;
  use petgraph::visit::IntoNodeReferences;

  use crate::test_utils::graph::expect_edge;

  use super::*;

  fn find_root(graph: &SimplifiedAssetGraph) -> NodeIndex {
    graph
      .node_references()
      .find_map(|(idx, node)| match node {
        SimplifiedAssetGraphNode::Root => Some(idx),
        _ => None,
      })
      .expect("root node not found")
  }

  #[test]
  fn test_build_empty_graph_has_only_root() {
    let builder = simplified_asset_graph_builder();
    let graph = builder.build();

    assert_eq!(graph.node_count(), 1);
    assert_eq!(graph.edge_count(), 0);
    let (_, node) = graph
      .node_references()
      .next()
      .expect("graph should contain one node");
    assert!(matches!(node, SimplifiedAssetGraphNode::Root));
  }

  #[test]
  fn test_entry_asset_adds_entry_edge_from_root() {
    let mut builder = simplified_asset_graph_builder();
    let a = builder.entry_asset("src/a.ts");
    let graph = builder.build();

    let root = find_root(&graph);

    assert_eq!(graph.node_count(), 2);
    assert_eq!(graph.edge_count(), 1);
    assert!(matches!(
      expect_edge(&graph, root, a).weight(),
      SimplifiedAssetGraphEdge::EntryAssetRoot(_)
    ));
  }

  #[test]
  fn test_asset_adds_node_without_edges() {
    let mut builder = simplified_asset_graph_builder();
    let _a = builder.asset("src/a.ts");
    let graph = builder.build();

    assert_eq!(graph.node_count(), 2);
    assert_eq!(graph.edge_count(), 0);
  }

  #[test]
  fn test_sync_dependency_adds_asset_dependency_edge() {
    let mut builder = simplified_asset_graph_builder();
    let a = builder.asset("src/a.ts");
    let b = builder.asset("src/b.ts");
    builder.sync_dependency(a, b);
    let graph = builder.build();

    assert_eq!(graph.node_count(), 3);
    assert_eq!(graph.edge_count(), 1);
    assert!(matches!(
      expect_edge(&graph, a, b).weight(),
      SimplifiedAssetGraphEdge::AssetDependency(_)
    ));
  }

  #[test]
  fn test_async_dependency_adds_asset_and_root_edges() {
    let mut builder = simplified_asset_graph_builder();
    let a = builder.asset("src/a.ts");
    let b = builder.asset("src/b.ts");
    builder.async_dependency(a, b);
    let graph = builder.build();
    let root = find_root(&graph);

    assert_eq!(graph.node_count(), 3);
    assert_eq!(graph.edge_count(), 2);
    assert!(matches!(
      expect_edge(&graph, a, b).weight(),
      SimplifiedAssetGraphEdge::AssetAsyncDependency(_)
    ));
    assert!(matches!(
      expect_edge(&graph, root, b).weight(),
      SimplifiedAssetGraphEdge::AsyncRoot(_)
    ));
  }

  #[test]
  fn test_entry_plus_async_dependency_has_three_edges_total() {
    let mut builder = simplified_asset_graph_builder();
    let a = builder.entry_asset("src/a.ts");
    let b = builder.asset("src/b.ts");
    builder.async_dependency(a, b);
    let graph = builder.build();
    let root = find_root(&graph);

    assert_eq!(graph.node_count(), 3);
    assert_eq!(graph.edge_count(), 3);
    assert!(matches!(
      expect_edge(&graph, root, a).weight(),
      SimplifiedAssetGraphEdge::EntryAssetRoot(_)
    ));
    assert!(matches!(
      expect_edge(&graph, a, b).weight(),
      SimplifiedAssetGraphEdge::AssetAsyncDependency(_)
    ));
    assert!(matches!(
      expect_edge(&graph, root, b).weight(),
      SimplifiedAssetGraphEdge::AsyncRoot(_)
    ));
  }

  #[test]
  fn test_asset_file_paths_match_inputs() {
    let mut builder = simplified_asset_graph_builder();
    let a = builder.asset("src/a.ts");
    let b = builder.entry_asset("src/b.ts");
    let graph = builder.build();

    let a_weight = graph.node_weight(a).unwrap();
    let b_weight = graph.node_weight(b).unwrap();

    match a_weight {
      SimplifiedAssetGraphNode::Asset(asset_ref) => {
        assert_eq!(asset_ref.file_path(), &PathBuf::from("src/a.ts"));
      }
      _ => panic!("node a is not an asset"),
    }

    match b_weight {
      SimplifiedAssetGraphNode::Asset(asset_ref) => {
        assert_eq!(asset_ref.file_path(), &PathBuf::from("src/b.ts"));
      }
      _ => panic!("node b is not an asset"),
    }
  }
}
