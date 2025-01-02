use std::{collections::HashSet, sync::Arc};

use petgraph::graph::NodeIndex;
use petgraph::stable_graph::StableDiGraph;
use petgraph::visit::EdgeRef;
use petgraph::visit::IntoEdgeReferences;
use petgraph::Direction;

use crate::types::Asset;
use crate::types::Dependency;

#[derive(Clone, Debug, PartialEq)]
pub struct AssetNode {
  pub asset: Asset,
  pub requested_symbols: HashSet<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DependencyNode {
  pub dependency: Arc<Dependency>,
  pub requested_symbols: HashSet<String>,
  pub state: DependencyState,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DependencyState {
  New,
  Deferred,
  Excluded,
  Resolved,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AssetGraphNode {
  Root,
  Entry,
  Asset(AssetNode),
  Dependency(DependencyNode),
}

#[derive(Clone, Debug)]
pub struct AssetGraph {
  pub graph: StableDiGraph<AssetGraphNode, ()>,
  root_node_index: NodeIndex,
}

impl Default for AssetGraph {
  fn default() -> Self {
    Self::new()
  }
}

impl AssetGraph {
  pub fn new() -> Self {
    let mut graph = StableDiGraph::new();
    let root_node_index = graph.add_node(AssetGraphNode::Root);
    AssetGraph {
      graph,
      root_node_index,
    }
  }

  pub fn edges(&self) -> Vec<u32> {
    let raw_edges = self.graph.edge_references();
    let mut edges = Vec::new();

    for edge in raw_edges {
      edges.push(edge.source().index() as u32);
      edges.push(edge.target().index() as u32);
    }

    edges
  }

  pub fn nodes(&self) -> impl Iterator<Item = &AssetGraphNode> {
    self.graph.node_weights()
  }

  pub fn nodes_from(&self, node_index: &NodeIndex) -> Vec<&AssetGraphNode> {
    let mut result = vec![];

    for edge in self.graph.edges_directed(*node_index, Direction::Outgoing) {
      let target_nx = edge.target();
      let target = self.graph.node_weight(target_nx).unwrap();
      result.push(target);
    }

    result
  }

  pub fn root_node(&self) -> NodeIndex {
    self.root_node_index
  }

  pub fn add_asset(&mut self, asset: Asset) -> NodeIndex {
    self.graph.add_node(AssetGraphNode::Asset(AssetNode {
      asset,
      requested_symbols: HashSet::default(),
    }))
  }

  pub fn get_asset_node(&self, nx: &NodeIndex) -> Option<&AssetNode> {
    let value = self.graph.node_weight(*nx)?;
    let AssetGraphNode::Asset(asset_node) = value else {
      return None;
    };
    Some(asset_node)
  }

  pub fn get_asset_node_mut(&mut self, nx: &NodeIndex) -> Option<&mut AssetNode> {
    let value = self.graph.node_weight_mut(*nx)?;
    let AssetGraphNode::Asset(asset_node) = value else {
      return None;
    };
    Some(asset_node)
  }

  pub fn get_asset_nodes(&self) -> Vec<&AssetNode> {
    let mut results = vec![];
    for n in self.nodes() {
      let AssetGraphNode::Asset(asset) = n else {
        continue;
      };
      results.push(asset);
    }
    results
  }

  pub fn add_dependency(&mut self, dependency: Dependency) -> NodeIndex {
    self
      .graph
      .add_node(AssetGraphNode::Dependency(DependencyNode {
        dependency: Arc::new(dependency),
        requested_symbols: HashSet::default(),
        state: DependencyState::New,
      }))
  }

  pub fn get_dependency_node(&self, nx: &NodeIndex) -> Option<&DependencyNode> {
    let value = self.graph.node_weight(*nx)?;
    let AssetGraphNode::Dependency(node) = value else {
      return None;
    };
    Some(node)
  }

  pub fn get_dependency_nodes(&self) -> Vec<&DependencyNode> {
    let mut results = vec![];
    for n in self.nodes() {
      let AssetGraphNode::Dependency(dependency) = n else {
        continue;
      };
      results.push(dependency);
    }
    results
  }

  pub fn get_dependency_node_mut(&mut self, nx: &NodeIndex) -> Option<&mut DependencyNode> {
    let value = self.graph.node_weight_mut(*nx)?;
    let AssetGraphNode::Dependency(node) = value else {
      return None;
    };
    Some(node)
  }

  pub fn add_entry_dependency(&mut self, dependency: Dependency) -> NodeIndex {
    let is_library = dependency.env.is_library;
    let dependency_nx = self.add_dependency(dependency);
    self.add_edge(&self.root_node_index.clone(), &dependency_nx);

    if is_library {
      if let Some(dependency_node) = self.get_dependency_node_mut(&dependency_nx) {
        dependency_node.requested_symbols.insert("*".into());
      }
    }

    dependency_nx
  }

  pub fn add_edge(&mut self, from_nx: &NodeIndex, to_nx: &NodeIndex) {
    self.graph.add_edge(*from_nx, *to_nx, ());
  }
}

impl PartialEq for AssetGraph {
  fn eq(&self, other: &Self) -> bool {
    let nodes = self.nodes();
    let other_nodes = other.nodes();

    let edges = self.edges();
    let other_edges = other.edges();

    nodes.eq(other_nodes) && edges.eq(&other_edges)
  }
}

impl std::hash::Hash for AssetGraph {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    for node in self.graph.node_weights() {
      std::mem::discriminant(node).hash(state);
      match node {
        AssetGraphNode::Asset(asset_node) => asset_node.asset.id.hash(state),
        AssetGraphNode::Dependency(dependency_node) => dependency_node.dependency.id().hash(state),
        _ => {}
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use crate::types::Symbol;
  use crate::types::Target;

  use super::super::propagate_requested_symbols::propagate_requested_symbols;
  use super::*;

  type TestSymbol<'a> = (&'a str, &'a str, bool);
  fn symbol(test_symbol: &TestSymbol) -> Symbol {
    let (local, exported, is_weak) = test_symbol;
    Symbol {
      local: String::from(*local),
      exported: String::from(*exported),
      is_weak: is_weak.to_owned(),
      ..Symbol::default()
    }
  }

  fn assert_requested_symbols(graph: &AssetGraph, nx: NodeIndex, expected: Vec<&str>) {
    assert_eq!(
      graph.get_dependency_node(&nx).unwrap().requested_symbols,
      expected
        .into_iter()
        .map(|s| s.into())
        .collect::<HashSet<String>>()
    );
  }

  fn add_asset(
    graph: &mut AssetGraph,
    parent_node: NodeIndex,
    symbols: Vec<TestSymbol>,
    file_path: &str,
  ) -> NodeIndex {
    let index_asset = Asset {
      file_path: PathBuf::from(file_path),
      symbols: Some(symbols.iter().map(symbol).collect()),
      ..Asset::default()
    };
    let asset_nid = graph.add_asset(index_asset);
    graph.add_edge(&parent_node, &asset_nid);
    asset_nid
  }

  fn add_dependency(
    graph: &mut AssetGraph,
    parent_node: NodeIndex,
    symbols: Vec<TestSymbol>,
  ) -> NodeIndex {
    let dep = Dependency {
      symbols: Some(symbols.iter().map(symbol).collect()),
      ..Dependency::default()
    };
    let node_index = graph.add_dependency(dep);
    graph.add_edge(&parent_node, &node_index);
    node_index
  }

  #[test]
  fn should_request_entry_asset() {
    let mut requested = HashSet::new();
    let mut graph = AssetGraph::new();
    let target = Target::default();
    let dep = Dependency::entry(String::from("index.js"), target);
    let entry_dep_node = graph.add_entry_dependency(dep);

    let index_asset_node = add_asset(&mut graph, entry_dep_node, vec![], "index.js");
    let dep_a_node = add_dependency(&mut graph, index_asset_node, vec![("a", "a", false)]);

    for (dependency_node_index, _) in
      propagate_requested_symbols(&mut graph, index_asset_node, entry_dep_node).unwrap()
    {
      requested.insert(dependency_node_index);
    }

    assert_eq!(requested, HashSet::from_iter(vec![dep_a_node]));
    assert_requested_symbols(&graph, dep_a_node, vec!["a"]);
  }

  #[test]
  fn should_propagate_named_reexports() {
    let mut graph = AssetGraph::new();
    let target = Target::default();
    let dep = Dependency::entry(String::from("index.js"), target);
    let entry_dep_node = graph.add_entry_dependency(dep);

    // entry.js imports "a" from library.js
    let entry_asset_node = add_asset(&mut graph, entry_dep_node, vec![], "entry.js");
    let library_dep_node = add_dependency(&mut graph, entry_asset_node, vec![("a", "a", false)]);
    propagate_requested_symbols(&mut graph, entry_asset_node, entry_dep_node).unwrap();

    // library.js re-exports "a" from a.js and "b" from b.js
    // only "a" is used in entry.js
    let library_asset_node = add_asset(
      &mut graph,
      library_dep_node,
      vec![("a", "a", true), ("b", "b", true)],
      "library.js",
    );
    let a_dep = add_dependency(&mut graph, library_asset_node, vec![("a", "a", true)]);
    let b_dep = add_dependency(&mut graph, library_asset_node, vec![("b", "b", true)]);

    let mut requested_deps = Vec::new();

    for (dependency_node_index, _) in
      propagate_requested_symbols(&mut graph, library_asset_node, library_dep_node).unwrap()
    {
      requested_deps.push(dependency_node_index);
    }

    assert_eq!(
      requested_deps,
      vec![b_dep, a_dep],
      "Should request both new deps"
    );

    // "a" should be the only requested symbol
    assert_requested_symbols(&graph, library_dep_node, vec!["a"]);
    assert_requested_symbols(&graph, a_dep, vec!["a"]);
    assert_requested_symbols(&graph, b_dep, vec![]);
  }

  #[test]
  fn should_propagate_wildcard_reexports() {
    let mut graph = AssetGraph::new();
    let target = Target::default();
    let dep = Dependency::entry(String::from("index.js"), target);
    let entry_dep_node = graph.add_entry_dependency(dep);

    // entry.js imports "a" from library.js
    let entry_asset_node = add_asset(&mut graph, entry_dep_node, vec![], "entry.js");
    let library_dep_node = add_dependency(&mut graph, entry_asset_node, vec![("a", "a", false)]);
    propagate_requested_symbols(&mut graph, entry_asset_node, entry_dep_node).unwrap();

    // library.js re-exports "*" from a.js and "*" from b.js
    // only "a" is used in entry.js
    let library_asset_node = add_asset(&mut graph, library_dep_node, vec![], "library.js");
    let a_dep = add_dependency(&mut graph, library_asset_node, vec![("*", "*", true)]);
    let b_dep = add_dependency(&mut graph, library_asset_node, vec![("*", "*", true)]);

    let mut requested_deps = Vec::new();
    for (dependency_node_index, _) in
      propagate_requested_symbols(&mut graph, library_asset_node, library_dep_node).unwrap()
    {
      requested_deps.push(dependency_node_index);
    }
    assert_eq!(
      requested_deps,
      vec![b_dep, a_dep],
      "Should request both new deps"
    );

    // "a" should be marked as requested on all deps as wildcards make it
    // unclear who the owning dep is
    assert_requested_symbols(&graph, library_dep_node, vec!["a"]);
    assert_requested_symbols(&graph, a_dep, vec!["a"]);
    assert_requested_symbols(&graph, b_dep, vec!["a"]);
  }

  #[test]
  fn should_propagate_nested_reexports() {
    let mut graph = AssetGraph::new();
    let target = Target::default();
    let dep = Dependency::entry(String::from("index.js"), target);
    let entry_dep_node = graph.add_entry_dependency(dep);

    // entry.js imports "a" from library
    let entry_asset_node = add_asset(&mut graph, entry_dep_node, vec![], "entry.js");
    let library_dep_node = add_dependency(&mut graph, entry_asset_node, vec![("a", "a", false)]);
    propagate_requested_symbols(&mut graph, entry_asset_node, entry_dep_node).unwrap();

    // library.js re-exports "*" from library/index.js
    let library_entry_asset_node = add_asset(&mut graph, library_dep_node, vec![], "library.js");
    let library_reexport_dep_node =
      add_dependency(&mut graph, library_entry_asset_node, vec![("*", "*", true)]);
    propagate_requested_symbols(&mut graph, library_entry_asset_node, library_dep_node).unwrap();

    // library/index.js re-exports "a" from a.js
    let library_asset_node = add_asset(
      &mut graph,
      library_reexport_dep_node,
      vec![("a", "a", true)],
      "library/index.js",
    );
    let a_dep = add_dependency(&mut graph, library_asset_node, vec![("a", "a", true)]);
    propagate_requested_symbols(&mut graph, library_entry_asset_node, library_dep_node).unwrap();

    // "a" should be marked as requested on all deps until the a dep is reached
    assert_requested_symbols(&graph, library_dep_node, vec!["a"]);
    assert_requested_symbols(&graph, library_reexport_dep_node, vec!["a"]);
    assert_requested_symbols(&graph, a_dep, vec!["a"]);
  }

  #[test]
  fn should_propagate_renamed_reexports() {
    let mut graph = AssetGraph::new();
    let target = Target::default();
    let dep = Dependency::entry(String::from("index.js"), target);
    let entry_dep_node = graph.add_entry_dependency(dep);

    // entry.js imports "a" from library
    let entry_asset_node = add_asset(&mut graph, entry_dep_node, vec![], "entry.js");
    let library_dep_node = add_dependency(&mut graph, entry_asset_node, vec![("a", "a", false)]);
    propagate_requested_symbols(&mut graph, entry_asset_node, entry_dep_node).unwrap();

    // library.js re-exports "b" from b.js renamed as "a"
    let library_asset_node = add_asset(
      &mut graph,
      library_dep_node,
      vec![("b", "a", true)],
      "library.js",
    );
    let b_dep = add_dependency(&mut graph, library_asset_node, vec![("b", "b", true)]);
    propagate_requested_symbols(&mut graph, library_asset_node, library_dep_node).unwrap();

    // "a" should be marked as requested on the library dep
    assert_requested_symbols(&graph, library_dep_node, vec!["a"]);
    // "b" should be marked as requested on the b dep
    assert_requested_symbols(&graph, b_dep, vec!["b"]);
  }

  #[test]
  fn should_propagate_namespace_reexports() {
    let mut graph = AssetGraph::new();
    let target = Target::default();
    let dep = Dependency::entry(String::from("index.js"), target);
    let entry_dep_node = graph.add_entry_dependency(dep);

    // entry.js imports "a" from library
    let entry_asset_node = add_asset(&mut graph, entry_dep_node, vec![], "entry.js");
    let library_dep_node = add_dependency(&mut graph, entry_asset_node, vec![("a", "a", false)]);
    propagate_requested_symbols(&mut graph, entry_asset_node, entry_dep_node).unwrap();

    // library.js re-exports "*" from stuff.js renamed as "a""
    // export * as a from './stuff.js'
    let library_asset_node = add_asset(
      &mut graph,
      library_dep_node,
      vec![("a", "a", true)],
      "library.js",
    );
    let stuff_dep = add_dependency(&mut graph, library_asset_node, vec![("a", "*", true)]);
    propagate_requested_symbols(&mut graph, library_asset_node, library_dep_node).unwrap();

    // "a" should be marked as requested on the library dep
    assert_requested_symbols(&graph, library_dep_node, vec!["a"]);
    // "*" should be marked as requested on the stuff dep
    assert_requested_symbols(&graph, stuff_dep, vec!["*"]);
  }
}
