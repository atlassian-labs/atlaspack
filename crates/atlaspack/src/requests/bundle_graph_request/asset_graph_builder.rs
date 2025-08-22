use std::path::PathBuf;

use atlaspack_core::{
  asset_graph::{AssetNode, DependencyNode},
  bundle_graph::AssetRef,
  types::Asset,
};
use petgraph::graph::NodeIndex;

use crate::requests::bundle_graph_request::{
  SimplifiedAssetGraph, SimplifiedAssetGraphEdge, SimplifiedAssetGraphNode,
};

pub struct AssetGraphBuilder {
  graph: SimplifiedAssetGraph,
  root: NodeIndex,
}

impl AssetGraphBuilder {
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
    let asset = self
      .graph
      .add_node(SimplifiedAssetGraphNode::Asset(AssetRef::new(
        AssetNode::from(Asset {
          file_path: PathBuf::from(path),
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
      SimplifiedAssetGraphEdge::AssetDependency(DependencyNode::default()),
    );
  }

  pub fn async_dependency(&mut self, source: NodeIndex, target: NodeIndex) {
    self.graph.add_edge(
      source,
      target,
      SimplifiedAssetGraphEdge::AssetAsyncDependency(DependencyNode::default()),
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

pub fn asset_graph_builder() -> AssetGraphBuilder {
  let mut graph = SimplifiedAssetGraph::new();
  let root = graph.add_node(SimplifiedAssetGraphNode::Root);
  AssetGraphBuilder { graph, root }
}

#[cfg(test)]
mod tests {
  use super::*;
}
