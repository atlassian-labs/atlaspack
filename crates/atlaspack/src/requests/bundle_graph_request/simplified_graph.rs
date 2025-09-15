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

impl std::fmt::Display for SimplifiedAssetGraphNode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      SimplifiedAssetGraphNode::Root => write!(f, "Root"),
      SimplifiedAssetGraphNode::Asset(asset_ref) => {
        write!(f, "Asset({})", asset_ref.file_path().display())
      }
    }
  }
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
        write!(f, "Asset({})", asset_node.file_path().display())
      }
    }
  }
}

#[derive(Clone, PartialEq, Debug)]
pub struct SimplifiedAssetDependency {
  dependency_node: DependencyNode,
  target_node_index: NodeIndex,
}

impl SimplifiedAssetDependency {
  pub fn new(dependency_node: DependencyNode, target_node_index: NodeIndex) -> Self {
    Self {
      dependency_node,
      target_node_index,
    }
  }

  pub fn target_node_index(&self) -> NodeIndex {
    self.target_node_index
  }

  pub fn dependency_node(&self) -> &DependencyNode {
    &self.dependency_node
  }
}

impl std::fmt::Display for SimplifiedAssetDependency {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.dependency_node)
  }
}

impl std::ops::Deref for SimplifiedAssetDependency {
  type Target = DependencyNode;

  fn deref(&self) -> &Self::Target {
    &self.dependency_node
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
  AssetDependency(SimplifiedAssetDependency),
  /// Asset to asset, means the asset is an async dependency of the other within a bundle
  ///
  /// If an asset A imports an asset B, of *different type*, then this edge is added.
  /// Not the dependency edge.
  AssetAsyncDependency(SimplifiedAssetDependency),
}

impl std::fmt::Display for SimplifiedAssetGraphEdge {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      SimplifiedAssetGraphEdge::EntryAssetRoot(dependency_node) => {
        write!(
          f,
          "EntryAssetRoot({})",
          dependency_node.dependency.specifier
        )
      }
      SimplifiedAssetGraphEdge::AsyncRoot(dependency_node) => {
        write!(f, "AsyncRoot({})", dependency_node.dependency.specifier)
      }
      SimplifiedAssetGraphEdge::TypeChangeRoot(dependency_node) => {
        write!(
          f,
          "TypeChangeRoot({})",
          dependency_node.dependency.specifier
        )
      }
      SimplifiedAssetGraphEdge::AssetDependency(simplified_asset_dependency) => {
        write!(
          f,
          "AssetDependency({})",
          simplified_asset_dependency.dependency.specifier
        )
      }
      SimplifiedAssetGraphEdge::AssetAsyncDependency(simplified_asset_dependency) => {
        write!(
          f,
          "AssetAsyncDependency({})",
          simplified_asset_dependency.dependency.specifier
        )
      }
    }
  }
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

  let root = asset_graph.root_node();

  let mut simplified_graph: StableDiGraph<SimplifiedAssetGraphNode, SimplifiedAssetGraphEdge> =
    asset_graph.graph.filter_map(
      |node_index, node| SimplifiedAssetGraphNode::try_from((node_index, node)).ok(),
      |_, _| None,
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
  //
  // For each edge in the graph, we check it is connecting a dependency to an asset.
  //
  // If the file types are different or if this is an async dependency, we add a new edge connecting
  // the asset directly to the root.
  for edge in asset_graph.graph.edge_references() {
    let source = edge.source();
    let target = edge.target();
    let source_node = asset_graph.get_node(&source).unwrap();
    let target_node = asset_graph.get_node(&target).unwrap();

    let AssetGraphNode::Dependency(dependency_node) = source_node else {
      continue;
    };
    assert!(matches!(target_node, AssetGraphNode::Asset(_)));

    let incoming_edges = asset_graph
      .graph
      .edges_directed(source, Direction::Incoming);
    for incoming_edge in incoming_edges {
      let incoming = incoming_edge.source();
      let incoming_node = asset_graph.get_node(&incoming);
      let Some(target_node) = asset_graph.get_node(&target) else {
        continue;
      };

      assert!(
        matches!(target_node, AssetGraphNode::Asset(_))
          || matches!(target_node, AssetGraphNode::Root),
        "Target node must be an asset or root"
      );

      if let (
        Some(AssetGraphNode::Asset(source_asset_node)),
        AssetGraphNode::Asset(target_asset_node),
      ) = (incoming_node, target_node)
      {
        if source_asset_node.asset.file_type != target_asset_node.asset.file_type {
          debug!(
          "Skipping dependency edge because source and target have different file types: {:?} -> {:?}",
          source_asset_node.asset.file_path, target_asset_node.asset.file_path
        );
          simplified_graph.add_edge(
            root,
            target,
            SimplifiedAssetGraphEdge::TypeChangeRoot(dependency_node.clone()),
          );

          // This becomes an async dependency regardless of priority
          simplified_graph.add_edge(
            incoming,
            target,
            SimplifiedAssetGraphEdge::AssetAsyncDependency(SimplifiedAssetDependency::new(
              dependency_node.clone(),
              target,
            )),
          );

          continue;
        }
      }

      if dependency_node.dependency.priority != Priority::Sync {
        simplified_graph.add_edge(
          root,
          target,
          SimplifiedAssetGraphEdge::AsyncRoot(dependency_node.clone()),
        );
        simplified_graph.add_edge(
          incoming,
          target,
          SimplifiedAssetGraphEdge::AssetAsyncDependency(SimplifiedAssetDependency::new(
            dependency_node.clone(),
            target,
          )),
        );
        continue;
      }

      if incoming == root {
        simplified_graph.add_edge(
          incoming,
          target,
          SimplifiedAssetGraphEdge::EntryAssetRoot(dependency_node.clone()),
        );
      } else {
        simplified_graph.add_edge(
          incoming,
          target,
          SimplifiedAssetGraphEdge::AssetDependency(SimplifiedAssetDependency::new(
            dependency_node.clone(),
            target,
          )),
        );
      }
    }
  }

  simplified_graph
}

#[cfg(test)]
mod tests {
  use std::path::Path;
  use std::{fs, path::PathBuf, process::Command};

  use atlaspack_core::types::FileType;
  use atlaspack_core::types::{Asset, Dependency};
  use petgraph::dot::Dot;

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
      self.graph.add_edge(self.graph.root_node(), dependency);
      self.graph.add_edge(dependency, asset);
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
      self.graph.add_edge(source, dependency);
      self.graph.add_edge(dependency, target);
    }

    fn async_dependency(&mut self, source: NodeIndex, target: NodeIndex) {
      let dependency = self.graph.add_dependency(Dependency {
        priority: Priority::Lazy,
        ..Dependency::default()
      });
      self.graph.add_edge(source, dependency);
      self.graph.add_edge(dependency, target);
    }

    fn build(self) -> AssetGraph {
      self.graph
    }
  }

  fn asset_graph_builder() -> AssetGraphBuilder {
    AssetGraphBuilder::new()
  }

  fn print_test_case(
    name: &str,
    input_graph: &AssetGraph,
    simplified_graph: &SimplifiedAssetGraph,
  ) {
    let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
      .join("src/requests/bundle_graph_request/simplified_graph/tests");
    fs::create_dir_all(&output_dir).unwrap();

    let input_graph: StableDiGraph<String, String> = input_graph
      .graph
      .map(|_, node| format!("{}", node), |_, _edge| "".to_string());
    let simplified_graph: StableDiGraph<String, String> =
      simplified_graph.map(|_, node| format!("{}", node), |_, edge| format!("{}", edge));

    let input_dot = format!("{}", Dot::new(&input_graph));
    let simplified_dot = format!("{}", Dot::new(&simplified_graph));

    let input_path = output_dir.join(format!("{}.INPUT.dot", name));
    let simplified_path = output_dir.join(format!("{}.OUTPUT.dot", name));

    fs::write(&input_path, input_dot).unwrap();
    fs::write(&simplified_path, simplified_dot).unwrap();

    fn run_dot(dot: &Path, output_path: &Path) {
      let mut cmd = Command::new("dot");
      cmd.arg("-Tpng").arg(dot).arg("-o").arg(output_path);
      println!("Running dot: {:?}", cmd);
      cmd.output().unwrap();
    }

    run_dot(&input_path, &input_path.with_extension("png"));
    run_dot(&simplified_path, &simplified_path.with_extension("png"));
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

    print_test_case(
      "test_simplify_graph_with_single_asset",
      &asset_graph,
      &simplified_graph,
    );
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

    print_test_case(
      "test_simplify_graph_with_sync_dependencies",
      &asset_graph,
      &simplified_graph,
    );
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

    print_test_case(
      "test_simplify_graph_with_async_dependencies",
      &asset_graph,
      &simplified_graph,
    );
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
    assert_eq!(simplified_graph.edge_count(), 3);

    assert!(matches!(
      expect_edge(&simplified_graph, root, a).weight(),
      SimplifiedAssetGraphEdge::EntryAssetRoot(_)
    ));
    assert!(matches!(
      expect_edge(&simplified_graph, root, b).weight(),
      SimplifiedAssetGraphEdge::TypeChangeRoot(_)
    ));
    assert!(matches!(
      expect_edge(&simplified_graph, a, b).weight(),
      SimplifiedAssetGraphEdge::AssetAsyncDependency(_)
    ));

    print_test_case(
      "test_type_change_creates_type_change_root_edge_and_skips_dependency",
      &asset_graph,
      &simplified_graph,
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
    assert_eq!(simplified_graph.edge_count(), 3);

    assert!(matches!(
      expect_edge(&simplified_graph, root, a).weight(),
      SimplifiedAssetGraphEdge::EntryAssetRoot(_)
    ));
    assert!(matches!(
      expect_edge(&simplified_graph, root, b).weight(),
      SimplifiedAssetGraphEdge::TypeChangeRoot(_)
    ));
    assert!(matches!(
      expect_edge(&simplified_graph, a, b).weight(),
      SimplifiedAssetGraphEdge::AssetAsyncDependency(_)
    ));

    // Ensure no async root edge on type change
    assert!(
      simplified_graph
        .edges_connecting(root, b)
        .find(|e| matches!(e.weight(), SimplifiedAssetGraphEdge::AsyncRoot(_)))
        .is_none(),
      "Unexpected AsyncRoot edge present to b"
    );

    print_test_case(
      "test_type_change_with_async_dependency_only_adds_type_change_root",
      &asset_graph,
      &simplified_graph,
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

    print_test_case(
      "test_multiple_entry_assets_create_multiple_root_entry_edges",
      &asset_graph,
      &simplified_graph,
    );
  }

  #[test]
  fn test_simplify_graph_with_nested_chain_and_multiple_children() {
    // graph {
    //   root -> entry -> a -> dep_a_b -> b
    //   b -> dep_b_c -> c
    //   b -> dep_b_d -> d
    // }
    //
    // becomes
    //
    // graph {
    //   root -> a
    //   a -> b
    //   b -> c
    //   b -> d
    // }
    let mut builder = asset_graph_builder();
    let a = builder.entry_asset("src/a.ts");
    let b = builder.asset("src/b.ts");
    let c = builder.asset("src/c.ts");
    let d = builder.asset("src/d.ts");
    builder.sync_dependency(a, b);
    builder.sync_dependency(b, c);
    builder.sync_dependency(b, d);
    let asset_graph = builder.build();
    let root = asset_graph.root_node();

    let simplified_graph = simplify_graph(&asset_graph);

    assert_eq!(simplified_graph.node_count(), 5);
    assert_eq!(simplified_graph.edge_count(), 4);

    assert!(matches!(
      expect_edge(&simplified_graph, root, a).weight(),
      SimplifiedAssetGraphEdge::EntryAssetRoot(_)
    ));
    assert!(matches!(
      expect_edge(&simplified_graph, a, b).weight(),
      SimplifiedAssetGraphEdge::AssetDependency(_)
    ));
    assert!(matches!(
      expect_edge(&simplified_graph, b, c).weight(),
      SimplifiedAssetGraphEdge::AssetDependency(_)
    ));
    assert!(matches!(
      expect_edge(&simplified_graph, b, d).weight(),
      SimplifiedAssetGraphEdge::AssetDependency(_)
    ));

    print_test_case(
      "test_simplify_graph_with_nested_chain_and_multiple_children",
      &asset_graph,
      &simplified_graph,
    );
  }
}
