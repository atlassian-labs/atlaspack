use std::collections::HashMap;

use atlaspack_core::bundle_graph::AssetRef;
use petgraph::{
  graph::NodeIndex,
  prelude::StableDiGraph,
  visit::{EdgeRef, IntoEdgeReferences},
};

use super::simplified_graph::*;

#[derive(Clone, PartialEq, Debug)]
pub enum AcyclicAssetGraphNode {
  Root,
  Asset(AssetRef),
  Cycle(Vec<AssetRef>),
}

impl AcyclicAssetGraphNode {
  pub fn file_path(&self) -> Option<&std::path::Path> {
    match self {
      AcyclicAssetGraphNode::Asset(asset_ref) => Some(asset_ref.file_path()),
      _ => None,
    }
  }
}

impl From<SimplifiedAssetGraphNode> for AcyclicAssetGraphNode {
  fn from(node: SimplifiedAssetGraphNode) -> Self {
    match node {
      SimplifiedAssetGraphNode::Root => AcyclicAssetGraphNode::Root,
      SimplifiedAssetGraphNode::Asset(asset_ref) => AcyclicAssetGraphNode::Asset(asset_ref),
    }
  }
}

pub type AcyclicAssetGraph = StableDiGraph<AcyclicAssetGraphNode, SimplifiedAssetGraphEdge>;

pub fn remove_cycles(graph: &SimplifiedAssetGraph) -> (NodeIndex, AcyclicAssetGraph) {
  let mut result: StableDiGraph<Option<AcyclicAssetGraphNode>, SimplifiedAssetGraphEdge> =
    graph.clone().map(|_, _| None, |_, edge| edge.clone());

  let sccs = petgraph::algo::tarjan_scc(graph);
  let mut root_node_index: Option<NodeIndex> = None;
  let mut index_map = HashMap::new();

  for scc in &sccs {
    if scc.len() == 1 {
      let node_index = scc[0];
      let node_weight = graph.node_weight(node_index).unwrap();
      let new_weight: AcyclicAssetGraphNode = AcyclicAssetGraphNode::from(node_weight.clone());
      let weight = result.node_weight_mut(node_index).unwrap();
      *weight = Some(new_weight);

      if node_weight == &SimplifiedAssetGraphNode::Root {
        root_node_index = Some(node_index);
      }
    } else {
      let assets = scc
        .iter()
        .map(|node_index| {
          let node_weight = graph.node_weight(*node_index).unwrap();
          match node_weight {
            SimplifiedAssetGraphNode::Asset(asset_node) => asset_node.clone(),
            _ => unreachable!(),
          }
        })
        .collect();

      let new_weight = AcyclicAssetGraphNode::Cycle(assets);
      let new_index = result.add_node(Some(new_weight));

      for node_index in scc {
        index_map.insert(node_index, new_index);
      }
    }
  }

  for edge in graph.edge_references() {
    let source_node_index = edge.source();
    let target_node_index = edge.target();

    // Being in the map means these
    let new_source_node_index = index_map.get(&source_node_index);
    let new_target_node_index = index_map.get(&target_node_index);

    let either_node_is_in_cycle =
      new_source_node_index.is_some() || new_target_node_index.is_some();
    if !either_node_is_in_cycle {
      continue;
    }

    let new_source_node_index = new_source_node_index.unwrap_or(&source_node_index);
    let new_target_node_index = new_target_node_index.unwrap_or(&target_node_index);

    if new_source_node_index == new_target_node_index {
      continue;
    }

    result.add_edge(
      *new_source_node_index,
      *new_target_node_index,
      edge.weight().clone(),
    );
  }

  for scc in &sccs {
    if scc.len() == 1 {
      continue;
    }

    for node_index in scc {
      result.remove_node(*node_index);
    }
  }

  let result = result.filter_map(
    |_, node_weight| node_weight.clone(),
    |_, edge| Some(edge.clone()),
  );

  (
    root_node_index.expect("The root can't cycle onto assets. Something is wrong"),
    result,
  )
}
