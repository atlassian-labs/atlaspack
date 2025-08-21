use atlaspack_core::{
  asset_graph::{AssetGraph, AssetGraphNode, DependencyNode},
  bundle_graph::AssetRef,
  types::Priority,
};
use petgraph::{
  prelude::StableDiGraph,
  visit::{EdgeRef, IntoEdgeReferences, IntoNodeReferences},
  Direction,
};
use tracing::debug;

#[derive(Clone, PartialEq)]
pub enum SimplifiedAssetGraphNode {
  Root,
  // Entry,
  Asset(AssetRef),
  None,
}

impl std::fmt::Debug for SimplifiedAssetGraphNode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      SimplifiedAssetGraphNode::Root => write!(f, "Root"),
      // SimplifiedAssetGraphNode::Entry => write!(f, "Entry"),
      SimplifiedAssetGraphNode::Asset(asset_node) => {
        write!(f, "Asset({:?})", asset_node.file_path())
      }
      SimplifiedAssetGraphNode::None => {
        write!(f, "None")
      }
    }
  }
}

#[derive(Clone, PartialEq, Debug)]
pub enum SimplifiedAssetGraphEdge {
  /// Root to asset, means the asset is an entry-point
  EntryAssetRoot(DependencyNode),
  /// Root to asset, means the asset is an async import
  AsyncRoot(DependencyNode),
  /// Root to asset, means the asset has been split due to type change
  TypeChangeRoot(DependencyNode),
  /// Asset to asset, means the asset is a dependency of the other within a bundle
  AssetDependency(DependencyNode),
  /// Asset to asset, means the asset is an async dependency of the other within a bundle
  AssetAsyncDependency(DependencyNode),
}

pub type SimplifiedAssetGraph = StableDiGraph<SimplifiedAssetGraphNode, SimplifiedAssetGraphEdge>;

pub fn simplify_graph(asset_graph: &AssetGraph) -> SimplifiedAssetGraph {
  {
    // TODO: Debugging code
    let root_node_index = asset_graph.root_node();
    let root_edges = asset_graph
      .graph
      .edges_directed(root_node_index, Direction::Outgoing);
    assert!(
      root_edges.count() >= 1,
      "Root node must have at least one outgoing edge"
    );
  }

  let mut simplified_graph = StableDiGraph::new();
  let mut root_node_index = None;

  let mut none_indexes = Vec::new();
  for (node_index, node) in asset_graph.graph.node_references() {
    match node {
      AssetGraphNode::Root => {
        root_node_index = Some(simplified_graph.add_node(SimplifiedAssetGraphNode::Root));
      }
      AssetGraphNode::Asset(asset_node) => {
        simplified_graph.add_node(SimplifiedAssetGraphNode::Asset(AssetRef::new(
          asset_node.clone(),
          node_index,
        )));
      }
      AssetGraphNode::Dependency(_dependency_node) => {
        let node_index = simplified_graph.add_node(SimplifiedAssetGraphNode::None);
        none_indexes.push(node_index);
      }
    };
  }
  // TODO: Do not crash if there is no root node
  let root_node_index = root_node_index.unwrap();

  for edge in asset_graph.graph.edge_references() {
    let source_node_index = edge.source();
    let target_node_index = edge.target();
    let source_node = asset_graph.get_node(&source_node_index).unwrap();
    let target_node = asset_graph.get_node(&target_node_index).unwrap();

    if let AssetGraphNode::Dependency(dependency_node) = source_node {
      assert!(matches!(target_node, AssetGraphNode::Asset(_)));

      for incoming_edge in asset_graph
        .graph
        .edges_directed(source_node_index, Direction::Incoming)
      {
        let incoming_node_index = incoming_edge.source();
        let target_node = asset_graph.get_node(&target_node_index).unwrap();

        assert!(
          matches!(target_node, AssetGraphNode::Asset(_))
            || matches!(target_node, AssetGraphNode::Root),
          "Target node must be an asset or root"
        );

        // The input graph looks like:
        //
        // a -> dependency_a_to_b -> b
        // a -> dependency_a_to_c -> c
        //
        // We are rewriting it into:
        //
        // a -> b
        // a -> c
        //
        // And storing the dependency as an edge weight.
        if let (
          Some(AssetGraphNode::Asset(source_asset_node)),
          AssetGraphNode::Asset(target_asset_node),
        ) = (asset_graph.get_node(&incoming_node_index), target_node)
        {
          if source_asset_node.asset.file_type != target_asset_node.asset.file_type {
            debug!(
              "Skipping dependency edge because source and target have different file types: {:?} -> {:?}",
              source_asset_node.asset.file_path, target_asset_node.asset.file_path
            );
            simplified_graph.add_edge(
              root_node_index,
              target_node_index,
              SimplifiedAssetGraphEdge::TypeChangeRoot(dependency_node.clone()),
            );
            continue;
          }
        }

        if dependency_node.dependency.priority != Priority::Sync {
          simplified_graph.add_edge(
            root_node_index,
            target_node_index,
            SimplifiedAssetGraphEdge::AsyncRoot(dependency_node.clone()),
          );
          simplified_graph.add_edge(
            incoming_node_index,
            target_node_index,
            SimplifiedAssetGraphEdge::AssetAsyncDependency(dependency_node.clone()),
          );
          continue;
        }

        if incoming_node_index == root_node_index {
          simplified_graph.add_edge(
            incoming_node_index,
            target_node_index,
            SimplifiedAssetGraphEdge::EntryAssetRoot(dependency_node.clone()),
          );
        } else {
          simplified_graph.add_edge(
            incoming_node_index,
            target_node_index,
            SimplifiedAssetGraphEdge::AssetDependency(dependency_node.clone()),
          );
        }
      }
    }
  }

  for node_index in none_indexes {
    simplified_graph.remove_node(node_index);
  }

  simplified_graph
}
