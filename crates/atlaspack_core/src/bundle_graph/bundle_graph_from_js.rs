use petgraph::{graph::NodeIndex, prelude::StableDiGraph, visit::Dfs};

use crate::{
  bundle_graph::bundle_graph::BundleGraph,
  types::{Bundle, BundleGraphEdgeType, BundleGraphNode},
};

pub struct BundleGraphFromJs {
  graph: StableDiGraph<BundleGraphNode, BundleGraphEdgeType>,
}
impl BundleGraphFromJs {
  pub fn new(nodes: Vec<BundleGraphNode>, edges: Vec<(u32, u32, BundleGraphEdgeType)>) -> Self {
    let mut graph = StableDiGraph::new();
    for node in nodes {
      graph.add_node(node);
    }
    for edge in edges {
      graph.add_edge(
        NodeIndex::new(edge.0 as usize),
        NodeIndex::new(edge.1 as usize),
        edge.2,
      );
    }
    BundleGraphFromJs { graph }
  }
}

impl BundleGraph for BundleGraphFromJs {
  // Temporary code just to validate functionality
  fn get_bundles(&self) -> Vec<&Bundle> {
    let mut bundles = Vec::new();
    let mut dfs = Dfs::new(&self.graph, NodeIndex::new(0));
    while let Some(node) = dfs.next(&self.graph) {
      let node = self.graph.node_weight(node).unwrap();
      if let BundleGraphNode::Bundle(node) = node {
        bundles.push(&node.value);
      }
    }
    bundles
  }
}
