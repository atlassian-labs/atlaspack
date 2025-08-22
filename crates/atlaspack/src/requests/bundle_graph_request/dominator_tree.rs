use std::{
  collections::{HashMap, HashSet},
  sync::Arc,
};

use async_trait::async_trait;
use atlaspack_core::{
  asset_graph::{AssetGraph, AssetGraphNode, DependencyNode},
  bundle_graph::{AssetRef, BundleGraph, BundleGraphBundle, BundleGraphEdge, BundleGraphNode},
  types::{Bundle, BundleBehavior, Environment, Target},
};
use petgraph::{
  graph::NodeIndex,
  prelude::StableDiGraph,
  visit::{Dfs, EdgeFiltered, EdgeRef, IntoNodeReferences},
  Direction,
};
use tracing::info;

use crate::{
  request_tracker::{Request, ResultAndInvalidations, RunRequestContext, RunRequestError},
  requests::{AssetGraphRequest, RequestResult},
};

pub type DominatorTree = StableDiGraph<DominatorTreeNode, DominatorTreeEdge>;

pub type DominatorTreeNode = AcyclicAssetGraphNode;

pub enum DominatorTreeEdge {
  ImmediateDominator,
  /// Root to asset, means the asset is an entry-point
  EntryAssetRoot(DependencyNode),
  /// Root to asset, means the asset is an async import
  AsyncRoot(DependencyNode),
  /// Root to asset, means the asset is a shared bundle
  SharedBundleRoot,
  /// Root to asset, means the asset has been split due to type change
  TypeChangeRoot(DependencyNode),
  /// Asset to asset, means the asset is a dependency of the other within a bundle
  AssetDependency(DependencyNode),
  /// Asset to asset, means the asset is an async dependency of the other within a bundle
  AssetAsyncDependency(DependencyNode),
}

impl From<SimplifiedAssetGraphEdge> for DominatorTreeEdge {
  fn from(edge: SimplifiedAssetGraphEdge) -> Self {
    match edge {
      SimplifiedAssetGraphEdge::EntryAssetRoot(dependency_node) => {
        DominatorTreeEdge::EntryAssetRoot(dependency_node)
      }
      SimplifiedAssetGraphEdge::AsyncRoot(dependency_node) => {
        DominatorTreeEdge::AsyncRoot(dependency_node)
      }
      SimplifiedAssetGraphEdge::TypeChangeRoot(dependency_node) => {
        DominatorTreeEdge::TypeChangeRoot(dependency_node)
      }
      SimplifiedAssetGraphEdge::AssetDependency(dependency_node) => {
        DominatorTreeEdge::AssetDependency(dependency_node)
      }
      SimplifiedAssetGraphEdge::AssetAsyncDependency(dependency_node) => {
        DominatorTreeEdge::AssetAsyncDependency(dependency_node)
      }
    }
  }
}

pub fn build_dominator_tree(graph: &AcyclicAssetGraph, root_id: NodeIndex) -> DominatorTree {
  let dominators = petgraph::algo::dominators::simple_fast(graph, root_id);
  let mut result = graph.map(
    |_, node| node.clone(),
    |_, edge| DominatorTreeEdge::from(edge.clone()),
  );

  for node_index in graph.node_indices() {
    let Some(immediate_dominator) = dominators.immediate_dominator(node_index) else {
      continue;
    };

    if immediate_dominator == root_id && !result.contains_edge(immediate_dominator, node_index) {
      result.add_edge(
        immediate_dominator,
        node_index,
        DominatorTreeEdge::SharedBundleRoot,
      );
    }

    result.add_edge(
      immediate_dominator,
      node_index,
      DominatorTreeEdge::ImmediateDominator,
    );
  }

  result
}
