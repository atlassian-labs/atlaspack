use atlaspack_core::asset_graph::DependencyNode;
use petgraph::{graph::NodeIndex, prelude::StableDiGraph};

use crate::requests::bundle_graph_request::{
  acyclic_asset_graph::{AcyclicAssetGraph, AcyclicAssetGraphNode},
  simplified_graph::SimplifiedAssetGraphEdge,
};

pub type DominatorTree = StableDiGraph<DominatorTreeNode, DominatorTreeEdge>;

pub type DominatorTreeNode = AcyclicAssetGraphNode;

#[derive(Debug, PartialEq)]
pub enum DominatorTreeEdge {
  /// Source is the immediate dominator of the target
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

impl std::fmt::Display for DominatorTreeEdge {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      DominatorTreeEdge::ImmediateDominator => write!(f, "ImmediateDominator"),
      DominatorTreeEdge::EntryAssetRoot(dependency_node) => {
        write!(f, "EntryAssetRoot({})", dependency_node)
      }
      DominatorTreeEdge::AsyncRoot(dependency_node) => {
        write!(f, "AsyncRoot({})", dependency_node)
      }
      DominatorTreeEdge::SharedBundleRoot => write!(f, "SharedBundleRoot"),
      DominatorTreeEdge::TypeChangeRoot(dependency_node) => {
        write!(f, "TypeChangeRoot({})", dependency_node)
      }
      DominatorTreeEdge::AssetDependency(dependency_node) => {
        write!(f, "AssetDependency({})", dependency_node)
      }
      DominatorTreeEdge::AssetAsyncDependency(dependency_node) => {
        write!(f, "AssetAsyncDependency({})", dependency_node)
      }
    }
  }
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

/// Builds a dominator tree from a simplified asset graph, where each node is under its immediate dominator.
///
/// The original edges and node indexes are preserved.
///
/// New "shared bundle root" edges are added between the root and newly split sub-trees.
pub fn build_dominator_tree(graph: &AcyclicAssetGraph, root_id: NodeIndex) -> DominatorTree {
  let mut result = graph.map(
    |_, node| node.clone(),
    |_, edge| DominatorTreeEdge::from(edge.clone()),
  );

  let dominators = petgraph::algo::dominators::simple_fast(
    &graph.filter_map(
      |_, node| Some(node),
      |_, edge| {
        let edge = edge.clone();
        match edge {
          SimplifiedAssetGraphEdge::AsyncRoot(_) => None,
          _ => Some(edge),
        }
      },
    ),
    root_id,
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

#[cfg(test)]
mod tests {
  use crate::{
    requests::bundle_graph_request::{
      acyclic_asset_graph::remove_cycles, asset_graph_builder::simplified_asset_graph_builder,
    },
    test_utils::graph::expect_edge,
  };

  use super::*;

  #[test]
  fn test_build_dominator_tree_with_empty_graph() {
    let mut graph = simplified_asset_graph_builder();
    let graph = graph.build();
    let (root, graph) = remove_cycles(&graph);

    let dominator_tree = build_dominator_tree(&graph, root);

    assert_eq!(dominator_tree.node_count(), 1);
    assert_eq!(dominator_tree.edge_count(), 0);
    assert!(dominator_tree.contains_node(root));
  }

  macro_rules! contains_matching {
    ($vec:expr, $e:pat) => {
      $vec.iter().any(|edge| matches!(edge, $e))
    };
  }

  macro_rules! assert_contains_matching {
    ($vec:expr, $e:pat, $label:literal) => {
      assert!(
        contains_matching!($vec, $e),
        "Expected to find {} in {}",
        $label,
        $vec
          .iter()
          .map(|e| e.to_string())
          .collect::<Vec<_>>()
          .join(", ")
      );
    };
  }

  #[test]
  fn test_build_dominator_tree_with_single_asset() {
    let mut graph = simplified_asset_graph_builder();
    let a = graph.entry_asset("a.js");
    let graph = graph.build();
    let (root, graph) = remove_cycles(&graph);

    let dominator_tree = build_dominator_tree(&graph, root);

    assert_eq!(dominator_tree.node_count(), 2);
    assert_eq!(dominator_tree.edge_count(), 2);
    assert!(dominator_tree.contains_node(root));
    assert!(dominator_tree.contains_node(a));
    assert!(dominator_tree.contains_edge(root, a));

    let edges = dominator_tree
      .edges_connecting(root, a)
      .map(|e| e.weight())
      .collect::<Vec<_>>();

    assert_eq!(edges.len(), 2);

    assert_contains_matching!(
      edges,
      DominatorTreeEdge::EntryAssetRoot(_),
      "EntryAssetRoot"
    );
    assert_contains_matching!(
      edges,
      DominatorTreeEdge::ImmediateDominator,
      "ImmediateDominator"
    );
  }

  #[test]
  fn test_build_dominator_tree_with_sync_dependencies() {
    let mut graph = simplified_asset_graph_builder();
    let a = graph.entry_asset("a.js");
    let b = graph.asset("b.js");
    graph.sync_dependency(a, b);
    let graph = graph.build();
    let (root, graph) = remove_cycles(&graph);

    let dominator_tree = build_dominator_tree(&graph, root);

    assert_eq!(dominator_tree.node_count(), 3);
    assert_eq!(dominator_tree.edge_count(), 4);
    assert!(dominator_tree.contains_node(root));

    // root -> a
    {
      let edges = dominator_tree
        .edges_connecting(root, a)
        .map(|e| e.weight())
        .collect::<Vec<_>>();
      assert_eq!(edges.len(), 2);
      assert_contains_matching!(
        edges,
        DominatorTreeEdge::EntryAssetRoot(_),
        "EntryAssetRoot"
      );
      assert_contains_matching!(
        edges,
        DominatorTreeEdge::ImmediateDominator,
        "ImmediateDominator"
      );
    }

    // a -> b
    {
      let edges = dominator_tree
        .edges_connecting(a, b)
        .map(|e| e.weight())
        .collect::<Vec<_>>();
      assert_contains_matching!(
        edges,
        DominatorTreeEdge::ImmediateDominator,
        "ImmediateDominator"
      );
      assert_contains_matching!(
        edges,
        DominatorTreeEdge::AssetDependency(_),
        "AssetDependency"
      );
    }
  }

  #[test]
  fn test_build_dominator_tree_with_async_dependencies() {
    let mut graph = simplified_asset_graph_builder();
    let a = graph.entry_asset("a.js");
    let b = graph.asset("b.js");
    graph.async_dependency(a, b);
    let graph = graph.build();
    let (root, graph) = remove_cycles(&graph);

    let dominator_tree = build_dominator_tree(&graph, root);

    assert_eq!(dominator_tree.node_count(), 3);
    assert_eq!(dominator_tree.edge_count(), 5);
    assert!(dominator_tree.contains_node(root));

    // root -> a
    {
      let edges = dominator_tree
        .edges_connecting(root, a)
        .map(|e| e.weight())
        .collect::<Vec<_>>();
      assert_eq!(edges.len(), 2);
      assert_contains_matching!(
        edges,
        DominatorTreeEdge::EntryAssetRoot(_),
        "EntryAssetRoot"
      );
      assert_contains_matching!(
        edges,
        DominatorTreeEdge::ImmediateDominator,
        "ImmediateDominator"
      );
    }

    // a -> b
    {
      let edges = dominator_tree
        .edges_connecting(a, b)
        .map(|e| e.weight())
        .collect::<Vec<_>>();
      assert_eq!(edges.len(), 2);
      assert_contains_matching!(
        edges,
        DominatorTreeEdge::AssetAsyncDependency(_),
        "AssetAsyncDependency"
      );
      assert_contains_matching!(
        edges,
        DominatorTreeEdge::ImmediateDominator,
        "ImmediateDominator"
      );
    }

    // root -> b
    {
      let edges = dominator_tree
        .edges_connecting(root, b)
        .map(|e| e.weight())
        .collect::<Vec<_>>();
      assert_contains_matching!(edges, DominatorTreeEdge::AsyncRoot(_), "AsyncRoot");
    }
  }

  #[test]
  fn test_build_dominator_tree_with_dependency_pulled_up() {
    // here we have:
    // graph {
    //   root -> a[label="entry"]
    //   root -> b[label="async root"]
    //
    //   a -> b[label="async"]
    //   a -> c
    //   b -> c
    // }
    //
    // The final dominator tree is
    // graph {
    //   root -> a
    //   a -> b
    //   a -> c
    // }
    //
    // When bundling, the bundler will skip the immediate dominator edge between a and b,
    // since b is also a root connected node.

    let mut graph = simplified_asset_graph_builder();
    let a = graph.entry_asset("a.js");
    let b = graph.asset("b.js");
    graph.async_dependency(a, b);
    let c = graph.asset("c.js");
    graph.sync_dependency(b, c);
    graph.sync_dependency(a, c);

    let graph = graph.build();
    let (root, graph) = remove_cycles(&graph);

    let dominator_tree = build_dominator_tree(&graph, root);

    assert_eq!(dominator_tree.node_count(), 4);
    // assert_eq!(dominator_tree.edge_count(), 9);
    assert!(dominator_tree.contains_node(root));

    // root -> a
    {
      let edges = dominator_tree
        .edges_connecting(root, a)
        .map(|e| e.weight())
        .collect::<Vec<_>>();
      assert_eq!(edges.len(), 2);
      assert_contains_matching!(
        edges,
        DominatorTreeEdge::EntryAssetRoot(_),
        "EntryAssetRoot"
      );
      assert_contains_matching!(
        edges,
        DominatorTreeEdge::ImmediateDominator,
        "ImmediateDominator"
      );
    }

    // a -> b
    // a -> c
    {
      let edges = dominator_tree
        .edges_connecting(a, b)
        .map(|e| e.weight())
        .collect::<Vec<_>>();
      assert_contains_matching!(
        edges,
        DominatorTreeEdge::AssetAsyncDependency(_),
        "AssetAsyncDependency"
      );

      let edges = dominator_tree
        .edges_connecting(a, c)
        .map(|e| e.weight())
        .collect::<Vec<_>>();
      // assert_eq!(edges.len(), 1);
      assert_contains_matching!(
        edges,
        DominatorTreeEdge::ImmediateDominator,
        "ImmediateDominator"
      );
    }

    // root -> b
    {
      let edges = dominator_tree
        .edges_connecting(root, b)
        .map(|e| e.weight())
        .collect::<Vec<_>>();
      assert_contains_matching!(edges, DominatorTreeEdge::AsyncRoot(_), "AsyncRoot");
    }
  }

  #[test]
  fn test_build_dominator_tree_with_nested_dependency() {
    // here we have:
    // graph {
    //   root -> a[label="entry"]
    //   root -> b[label="async root"]
    //
    //   a -> b[label="async"]
    //   b -> c
    // }
    //
    // The final dominator tree is
    // graph {
    //   root -> a
    //   a -> b
    //   b -> c
    // }
    //
    // When bundling, the bundler will skip the immediate dominator edge between a and b,
    // since b is also a root connected node.

    let mut graph = simplified_asset_graph_builder();
    let a = graph.entry_asset("a.js");
    let b = graph.asset("b.js");
    graph.async_dependency(a, b);
    let c = graph.asset("c.js");
    graph.sync_dependency(b, c);
    graph.sync_dependency(a, c);

    let graph = graph.build();
    let (root, graph) = remove_cycles(&graph);

    let dominator_tree = build_dominator_tree(&graph, root);

    assert_eq!(dominator_tree.node_count(), 4);
    // assert_eq!(dominator_tree.edge_count(), 9);
    assert!(dominator_tree.contains_node(root));

    // root -> a
    {
      let edges = dominator_tree
        .edges_connecting(root, a)
        .map(|e| e.weight())
        .collect::<Vec<_>>();
      assert_eq!(edges.len(), 2);
      assert_contains_matching!(
        edges,
        DominatorTreeEdge::EntryAssetRoot(_),
        "EntryAssetRoot"
      );
      assert_contains_matching!(
        edges,
        DominatorTreeEdge::ImmediateDominator,
        "ImmediateDominator"
      );
    }

    // a -> b
    // a -> c
    {
      let edges = dominator_tree
        .edges_connecting(a, b)
        .map(|e| e.weight())
        .collect::<Vec<_>>();
      assert_contains_matching!(
        edges,
        DominatorTreeEdge::AssetAsyncDependency(_),
        "AssetAsyncDependency"
      );

      let edges = dominator_tree
        .edges_connecting(a, c)
        .map(|e| e.weight())
        .collect::<Vec<_>>();
      // assert_eq!(edges.len(), 1);
      assert_contains_matching!(
        edges,
        DominatorTreeEdge::ImmediateDominator,
        "ImmediateDominator"
      );
    }

    // root -> b
    {
      let edges = dominator_tree
        .edges_connecting(root, b)
        .map(|e| e.weight())
        .collect::<Vec<_>>();
      assert_contains_matching!(edges, DominatorTreeEdge::AsyncRoot(_), "AsyncRoot");
    }
  }

  #[test]
  fn test_build_dominator_tree_with_nested_dependency_shared() {
    // here we have:
    // graph {
    //   root -> a[label="entry"]
    //   root -> b[label="async root"]
    //
    //   a -> b[label="async"]
    //   b -> d
    //   a -> c[label="async"]
    //   c -> d
    // }

    let mut graph = simplified_asset_graph_builder();
    let a = graph.entry_asset("a.js");
    let b = graph.asset("b.js");
    graph.async_dependency(a, b);
    let c = graph.asset("c.js");
    graph.async_dependency(a, c);

    let d = graph.asset("d.js");
    graph.sync_dependency(b, d);
    graph.sync_dependency(c, d);

    let graph = graph.build();
    let (root, graph) = remove_cycles(&graph);

    let dominator_tree = build_dominator_tree(&graph, root);

    assert!(dominator_tree.contains_node(root));

    // root -> a
    {
      let edges = dominator_tree
        .edges_connecting(root, a)
        .map(|e| e.weight())
        .collect::<Vec<_>>();
      assert_eq!(edges.len(), 2);
      assert_contains_matching!(
        edges,
        DominatorTreeEdge::EntryAssetRoot(_),
        "EntryAssetRoot"
      );
      assert_contains_matching!(
        edges,
        DominatorTreeEdge::ImmediateDominator,
        "ImmediateDominator"
      );
    }

    // a -> b
    // a -> c
    {
      let edges = dominator_tree
        .edges_connecting(a, b)
        .map(|e| e.weight())
        .collect::<Vec<_>>();
      assert_contains_matching!(
        edges,
        DominatorTreeEdge::AssetAsyncDependency(_),
        "AssetAsyncDependency"
      );

      let edges = dominator_tree
        .edges_connecting(a, c)
        .map(|e| e.weight())
        .collect::<Vec<_>>();
      // assert_eq!(edges.len(), 1);
      assert_contains_matching!(
        edges,
        DominatorTreeEdge::AssetAsyncDependency(_),
        "AssetAsyncDependency"
      );
    }

    // root -> b
    // root -> c
    {
      let edges = dominator_tree
        .edges_connecting(root, b)
        .map(|e| e.weight())
        .collect::<Vec<_>>();
      assert_contains_matching!(edges, DominatorTreeEdge::AsyncRoot(_), "AsyncRoot");
      let edges = dominator_tree
        .edges_connecting(root, c)
        .map(|e| e.weight())
        .collect::<Vec<_>>();
      assert_contains_matching!(edges, DominatorTreeEdge::AsyncRoot(_), "AsyncRoot");
    }

    // root -> d
    {
      let edges = dominator_tree
        .edges_connecting(root, d)
        .map(|e| e.weight())
        .collect::<Vec<_>>();
      assert_contains_matching!(
        edges,
        DominatorTreeEdge::SharedBundleRoot,
        "SharedBundleRoot"
      );
    }
  }
}
