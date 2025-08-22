use atlaspack_core::{
  asset_graph::{AssetGraph, AssetGraphNode, DependencyNode},
  bundle_graph::AssetRef,
  types::Priority,
};
use petgraph::{
  graph::NodeIndex,
  prelude::StableDiGraph,
  visit::{EdgeRef, IntoEdgeReferences},
  Direction,
};
use tracing::debug;

#[derive(Clone, PartialEq)]
pub enum SimplifiedAssetGraphNode {
  Root,
  Asset(AssetRef),
}

impl TryFrom<(NodeIndex, &AssetGraphNode)> for SimplifiedAssetGraphNode {
  type Error = ();

  fn try_from(value: (NodeIndex, &AssetGraphNode)) -> Result<Self, Self::Error> {
    let (node_index, node) = value;
    match node {
      AssetGraphNode::Root => Ok(SimplifiedAssetGraphNode::Root),
      AssetGraphNode::Asset(asset_node) => Ok(SimplifiedAssetGraphNode::Asset(AssetRef::new(
        asset_node.clone(),
        node_index,
      ))),
      AssetGraphNode::Dependency(_) => Err(()),
    }
  }
}

impl std::fmt::Debug for SimplifiedAssetGraphNode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      SimplifiedAssetGraphNode::Root => write!(f, "Root"),
      // SimplifiedAssetGraphNode::Entry => write!(f, "Entry"),
      SimplifiedAssetGraphNode::Asset(asset_node) => {
        write!(f, "Asset({:?})", asset_node.file_path())
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

  let root_node_index = asset_graph.root_node();

  let mut simplified_graph: StableDiGraph<SimplifiedAssetGraphNode, SimplifiedAssetGraphEdge> =
    asset_graph.graph.filter_map(
      |node_index, node| SimplifiedAssetGraphNode::try_from((node_index, node)).ok(),
      |_, _| None,
    );

  for edge in asset_graph.graph.edge_references() {
    let source_node_index = edge.source();
    let target_node_index = edge.target();
    let source_node = asset_graph.get_node(&source_node_index).unwrap();
    let target_node = asset_graph.get_node(&target_node_index).unwrap();

    let AssetGraphNode::Dependency(dependency_node) = source_node else {
      continue;
    };
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

  simplified_graph
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use atlaspack_core::types::FileType;
  use atlaspack_core::types::{Asset, Dependency};

  use crate::test_utils::graph::expect_edge;

  use super::*;

  struct AssetGraphBuilder {
    graph: AssetGraph,
  }

  impl AssetGraphBuilder {
    fn new() -> Self {
      Self {
        graph: AssetGraph::new(),
      }
    }

    fn entry_asset(&mut self, path: &str) -> NodeIndex {
      let asset = self.graph.add_asset(Asset {
        file_path: PathBuf::from(path),
        ..Asset::default()
      });
      let dependency = self.graph.add_entry_dependency(Dependency::default());
      self.graph.add_edge(&self.graph.root_node(), &dependency);
      self.graph.add_edge(&dependency, &asset);
      asset
    }

    fn asset(&mut self, path: &str) -> NodeIndex {
      let asset = self.graph.add_asset(Asset {
        file_path: PathBuf::from(path),
        ..Asset::default()
      });
      asset
    }

    fn sync_dependency(&mut self, source: NodeIndex, target: NodeIndex) {
      let dependency = self.graph.add_dependency(Dependency::default());
      self.graph.add_edge(&source, &dependency);
      self.graph.add_edge(&dependency, &target);
    }

    fn async_dependency(&mut self, source: NodeIndex, target: NodeIndex) {
      let dependency = self.graph.add_dependency(Dependency {
        priority: Priority::Lazy,
        ..Dependency::default()
      });
      self.graph.add_edge(&source, &dependency);
      self.graph.add_edge(&dependency, &target);
    }

    fn build(self) -> AssetGraph {
      self.graph
    }
  }

  fn asset_graph_builder() -> AssetGraphBuilder {
    AssetGraphBuilder::new()
  }

  #[test]
  fn test_simplify_graph_with_single_asset() {
    // graph {
    //   root -> entry -> asset
    // }
    //
    // becomes
    //
    // graph {
    //   root -> asset
    // }
    let mut asset_graph = asset_graph_builder();
    let asset = asset_graph.entry_asset("src/index.ts");
    let asset_graph = asset_graph.build();
    let root = asset_graph.root_node();

    let simplified_graph = simplify_graph(&asset_graph);

    assert_eq!(simplified_graph.node_count(), 2);
    assert_eq!(simplified_graph.edge_count(), 1);

    assert_eq!(
      simplified_graph.node_weight(root).unwrap(),
      &SimplifiedAssetGraphNode::Root
    );
    assert!(matches!(
      simplified_graph.node_weight(asset).unwrap(),
      SimplifiedAssetGraphNode::Asset(_)
    ));

    assert!(matches!(
      simplified_graph
        .edges_connecting(root, asset)
        .next()
        .unwrap()
        .weight(),
      SimplifiedAssetGraphEdge::EntryAssetRoot(_)
    ));
  }

  #[test]
  fn test_simplify_graph_with_sync_dependencies() {
    // graph {
    //   root -> entry -> a -> dep_a_b -> b
    // }
    //
    // becomes
    //
    // graph {
    //   root -> entry -> a -> b
    // }
    let mut asset_graph = asset_graph_builder();
    let a = asset_graph.entry_asset("src/a.ts");
    let b = asset_graph.asset("src/b.ts");
    asset_graph.sync_dependency(a, b);
    let asset_graph = asset_graph.build();
    let root = asset_graph.root_node();

    let simplified_graph = simplify_graph(&asset_graph);

    assert_eq!(simplified_graph.node_count(), 3);
    assert_eq!(simplified_graph.edge_count(), 2);

    assert_eq!(
      simplified_graph.node_weight(root).unwrap(),
      &SimplifiedAssetGraphNode::Root
    );
    assert!(matches!(
      simplified_graph.node_weight(a).unwrap(),
      SimplifiedAssetGraphNode::Asset(_)
    ));
    assert!(matches!(
      simplified_graph.node_weight(b).unwrap(),
      SimplifiedAssetGraphNode::Asset(_)
    ));

    assert!(matches!(
      expect_edge(&simplified_graph, root, a).weight(),
      SimplifiedAssetGraphEdge::EntryAssetRoot(_)
    ));
    assert!(matches!(
      expect_edge(&simplified_graph, a, b).weight(),
      SimplifiedAssetGraphEdge::AssetDependency(_)
    ));
  }

  #[test]
  fn test_simplify_graph_with_async_dependencies() {
    // graph {
    //   root -> entry -> a -> asyncdep_a_b -> b
    // }
    //
    // becomes
    //
    // graph {
    //   root -> entry -> a -> b
    // }
    let mut asset_graph = asset_graph_builder();
    let a = asset_graph.entry_asset("src/a.ts");
    let b = asset_graph.asset("src/b.ts");
    asset_graph.async_dependency(a, b);
    let asset_graph = asset_graph.build();
    let root = asset_graph.root_node();

    let simplified_graph = simplify_graph(&asset_graph);

    assert_eq!(simplified_graph.node_count(), 3);
    assert_eq!(simplified_graph.edge_count(), 3);

    assert_eq!(
      simplified_graph.node_weight(root).unwrap(),
      &SimplifiedAssetGraphNode::Root
    );
    assert!(matches!(
      simplified_graph.node_weight(a).unwrap(),
      SimplifiedAssetGraphNode::Asset(_)
    ));
    assert!(matches!(
      simplified_graph.node_weight(b).unwrap(),
      SimplifiedAssetGraphNode::Asset(_)
    ));

    assert!(matches!(
      expect_edge(&simplified_graph, root, a).weight(),
      SimplifiedAssetGraphEdge::EntryAssetRoot(_)
    ));
    assert!(matches!(
      expect_edge(&simplified_graph, a, b).weight(),
      SimplifiedAssetGraphEdge::AssetAsyncDependency(_)
    ));
    assert!(matches!(
      expect_edge(&simplified_graph, root, b).weight(),
      SimplifiedAssetGraphEdge::AsyncRoot(_)
    ));
  }

  #[test]
  fn test_type_change_creates_type_change_root_edge_and_skips_dependency() {
    // graph {
    //   root -> entry -> a (js) -> dep_a_b -> b (css)
    // }
    //
    // becomes
    //
    // graph {
    //   root -> a (entry)
    //   root -> b (type-change)
    // }
    let mut builder = asset_graph_builder();
    let a = builder.entry_asset("src/a.ts");
    let b = builder.asset("src/b.css");
    {
      // Force b to be CSS to trigger type-change logic (a defaults to JS)
      let b_node = builder.graph.get_asset_node_mut(&b).unwrap();
      b_node.asset.file_type = FileType::Css;
    }
    builder.sync_dependency(a, b);
    let asset_graph = builder.build();
    let root = asset_graph.root_node();

    let simplified_graph = simplify_graph(&asset_graph);

    assert_eq!(simplified_graph.node_count(), 3);
    assert_eq!(simplified_graph.edge_count(), 2);

    assert!(matches!(
      expect_edge(&simplified_graph, root, a).weight(),
      SimplifiedAssetGraphEdge::EntryAssetRoot(_)
    ));
    assert!(matches!(
      expect_edge(&simplified_graph, root, b).weight(),
      SimplifiedAssetGraphEdge::TypeChangeRoot(_)
    ));

    // Ensure there is no direct a -> b edge due to type change
    assert!(
      simplified_graph.edges_connecting(a, b).next().is_none(),
      "Unexpected a -> b edge present"
    );
  }

  #[test]
  fn test_type_change_with_async_dependency_only_adds_type_change_root() {
    // graph {
    //   root -> entry -> a (js) -> asyncdep_a_b -> b (css)
    // }
    //
    // becomes
    //
    // graph {
    //   root -> a (entry)
    //   root -> b (type-change)
    // }
    let mut builder = asset_graph_builder();
    let a = builder.entry_asset("src/a.ts");
    let b = builder.asset("src/b.css");
    {
      // Force b to be CSS to trigger type-change logic
      let b_node = builder.graph.get_asset_node_mut(&b).unwrap();
      b_node.asset.file_type = FileType::Css;
    }
    builder.async_dependency(a, b);
    let asset_graph = builder.build();
    let root = asset_graph.root_node();

    let simplified_graph = simplify_graph(&asset_graph);

    assert_eq!(simplified_graph.node_count(), 3);
    assert_eq!(simplified_graph.edge_count(), 2);

    assert!(matches!(
      expect_edge(&simplified_graph, root, a).weight(),
      SimplifiedAssetGraphEdge::EntryAssetRoot(_)
    ));
    assert!(matches!(
      expect_edge(&simplified_graph, root, b).weight(),
      SimplifiedAssetGraphEdge::TypeChangeRoot(_)
    ));

    // Ensure no async edges were added due to early continue on type change
    assert!(
      simplified_graph.edges_connecting(a, b).next().is_none(),
      "Unexpected async a -> b edge present"
    );
    assert!(
      simplified_graph
        .edges_connecting(root, b)
        .find(|e| matches!(e.weight(), SimplifiedAssetGraphEdge::AsyncRoot(_)))
        .is_none(),
      "Unexpected AsyncRoot edge present to b"
    );
  }

  #[test]
  fn test_multiple_entry_assets_create_multiple_root_entry_edges() {
    // graph {
    //   root -> entry -> a
    //   root -> entry -> b
    // }
    let mut builder = asset_graph_builder();
    let a = builder.entry_asset("src/a.ts");
    let b = builder.entry_asset("src/b.ts");
    let asset_graph = builder.build();
    let root = asset_graph.root_node();

    let simplified_graph = simplify_graph(&asset_graph);

    assert_eq!(simplified_graph.node_count(), 3);
    assert_eq!(simplified_graph.edge_count(), 2);

    assert!(matches!(
      expect_edge(&simplified_graph, root, a).weight(),
      SimplifiedAssetGraphEdge::EntryAssetRoot(_)
    ));
    assert!(matches!(
      expect_edge(&simplified_graph, root, b).weight(),
      SimplifiedAssetGraphEdge::EntryAssetRoot(_)
    ));
  }
}
