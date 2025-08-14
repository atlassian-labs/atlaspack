use std::collections::HashMap;

use petgraph::{
  prelude::StableDiGraph,
  visit::{EdgeRef, IntoEdgeReferences},
  Direction,
};

use crate::asset_graph::{AssetGraph, AssetGraphNode, AssetNode, DependencyNode};

pub struct BundleGraph {}

pub enum SimplifiedAssetGraphNode {
  Root,
  Entry,
  Asset(AssetNode),
  None,
}

pub enum SimplifiedAssetGraphEdge {
  Dependency(DependencyNode),
  None,
}

pub fn simplify_graph(
  graph: &AssetGraph,
) -> StableDiGraph<SimplifiedAssetGraphNode, SimplifiedAssetGraphEdge> {
  let mut simplified_graph = StableDiGraph::new();
  let mut asset_node_index_by_id = HashMap::new();

  for node in graph.nodes() {
    match node {
      AssetGraphNode::Root => {
        simplified_graph.add_node(SimplifiedAssetGraphNode::Root);
      }
      AssetGraphNode::Entry => {
        simplified_graph.add_node(SimplifiedAssetGraphNode::Entry);
      }
      AssetGraphNode::Asset(asset_node) => {
        let node_index =
          simplified_graph.add_node(SimplifiedAssetGraphNode::Asset(asset_node.clone()));
        asset_node_index_by_id.insert(asset_node.asset.id.clone(), node_index);
      }
      AssetGraphNode::Dependency(_dependency_node) => {
        simplified_graph.add_node(SimplifiedAssetGraphNode::None);
      }
    }
  }

  for edge in graph.graph.edge_references() {
    let source_node_index = edge.source();
    let target_node_index = edge.target();
    let source_node = graph.get_node(&source_node_index).unwrap();
    let target_node = graph.get_node(&target_node_index).unwrap();

    if let AssetGraphNode::Dependency(dependency_node) = source_node {
      assert!(matches!(target_node, AssetGraphNode::Asset(_)));

      for incoming_edge in graph
        .graph
        .edges_directed(source_node_index, Direction::Incoming)
      {
        let incoming_node_index = incoming_edge.source();
        let incoming_node = graph.get_node(&incoming_node_index).unwrap();

        if let AssetGraphNode::Asset(_asset_node) = incoming_node {
          simplified_graph.add_edge(
            incoming_node_index,
            target_node_index,
            SimplifiedAssetGraphEdge::Dependency(dependency_node.clone()),
          );
        }
      }
    }
  }

  simplified_graph
}

#[cfg(test)]
mod test {}
