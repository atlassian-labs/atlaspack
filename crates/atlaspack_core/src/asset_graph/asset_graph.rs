use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

use petgraph::Direction;
use petgraph::stable_graph::StableDiGraph;
use petgraph::visit::EdgeRef;
use petgraph::visit::IntoEdgeReferences;

use crate::types::Asset;
use crate::types::Dependency;

#[derive(Clone, Debug, PartialEq)]
pub enum DependencyState {
  New,
  Deferred,
  Excluded,
  Resolved,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum AssetGraphNode {
  Root,
  Entry,
  Asset(Arc<Asset>),
  Dependency(Arc<Dependency>),
}

pub type NodeId = usize;

#[derive(Clone, Debug)]
pub struct AssetGraph {
  pub graph: StableDiGraph<NodeId, ()>,
  nodes: Vec<AssetGraphNode>,
  requested_symbols: HashMap<NodeId, HashSet<String>>,
  dependency_states: HashMap<NodeId, DependencyState>,
  content_key_to_node_id: HashMap<String, NodeId>,
  node_id_to_node_index: HashMap<NodeId, NodeId>,
  root_node_id: NodeId,
  node_delta: Vec<NodeId>,
  pub starting_node_count: usize,
}

impl Default for AssetGraph {
  fn default() -> Self {
    Self::new()
  }
}

impl AssetGraph {
  pub fn new() -> Self {
    let mut graph = StableDiGraph::new();

    let mut node_id_to_node_index = HashMap::new();
    let nodes = vec![AssetGraphNode::Root];
    let root_node_id = 0;

    node_id_to_node_index.insert(root_node_id, graph.add_node(root_node_id));

    AssetGraph {
      graph,
      content_key_to_node_id: HashMap::new(),
      requested_symbols: HashMap::new(),
      dependency_states: HashMap::new(),
      node_id_to_node_index,
      nodes,
      root_node_id,
      node_delta: Vec::new(),
      starting_node_count: 0,
    }
  }

  pub fn edges(&self) -> Vec<u32> {
    let raw_edges = self.graph.edge_references();
    let mut edges = Vec::new();
    let nodes = self.graph.node_weights().collect::<Vec<_>>();

    for edge in raw_edges {
      edges.push(*nodes[edge.source().index()] as u32);
      edges.push(*nodes[edge.target().index()] as u32);
    }

    edges
  }

  pub fn nodes(&self) -> impl Iterator<Item = &AssetGraphNode> {
    self.nodes.iter()
  }

  pub fn new_nodes(&self) -> Vec<&AssetGraphNode> {
    self.nodes[self.starting_node_count..].iter().collect()
  }

  pub fn updated_nodes(&self) -> Vec<&AssetGraphNode> {
    self
      .node_delta
      .iter()
      .map(|node_id| &self.nodes[*node_id])
      .collect()
  }

  pub fn root_node(&self) -> NodeId {
    self.root_node_id
  }

  pub fn get_node(&self, idx: &NodeId) -> Option<&AssetGraphNode> {
    self.nodes.get(*idx)
  }

  pub fn get_node_mut(&mut self, idx: &NodeId) -> Option<&mut AssetGraphNode> {
    self.nodes.get_mut(*idx)
  }

  pub fn is_diff(&self, a: &AssetGraphNode, b: &AssetGraphNode) -> bool {
    a != b
  }

  fn add_node(&mut self, content_key: String, node: AssetGraphNode) -> NodeId {
    let node_id = if let Some(existing_node_id) = self.content_key_to_node_id.get(&content_key) {
      self.nodes[*existing_node_id] = node;
      self.node_delta.push(*existing_node_id);

      *existing_node_id
    } else {
      let node_id = self.nodes.len();
      self.nodes.push(node);
      self
        .content_key_to_node_id
        .insert(content_key.clone(), node_id);
      node_id
    };

    let node_index = self.graph.add_node(node_id);
    self.node_id_to_node_index.insert(node_id, node_index);

    node_id
  }

  pub fn add_asset(&mut self, asset: Arc<Asset>) -> NodeId {
    let node_id = self.add_node(asset.id.clone(), AssetGraphNode::Asset(asset));
    self.requested_symbols.insert(node_id, HashSet::new());
    node_id
  }

  pub fn get_asset_node(&self, idx: &NodeId) -> Option<&Asset> {
    let value = self.get_node(idx)?;
    let AssetGraphNode::Asset(asset_node) = value else {
      return None;
    };
    Some(asset_node)
  }

  pub fn add_dependency(&mut self, dependency: Dependency) -> NodeId {
    let node_id = self.add_node(
      dependency.id(),
      AssetGraphNode::Dependency(Arc::new(dependency)),
    );

    self.requested_symbols.insert(node_id, HashSet::new());
    self.dependency_states.insert(node_id, DependencyState::New);
    node_id
  }

  pub fn get_dependency_node(&self, idx: &NodeId) -> Option<&Dependency> {
    let value = self.get_node(idx)?;
    let AssetGraphNode::Dependency(node) = value else {
      return None;
    };
    Some(node)
  }

  pub fn get_dependency_nodes(&self) -> Vec<&Dependency> {
    let mut results = vec![];
    for n in self.nodes() {
      let AssetGraphNode::Dependency(dependency) = n else {
        continue;
      };
      results.push(dependency.as_ref());
    }
    results
  }

  pub fn add_entry_dependency(&mut self, dependency: Dependency) -> NodeId {
    let is_library = dependency.env.is_library;
    let dependency_idx = self.add_dependency(dependency);

    if is_library {
      self
        .requested_symbols
        .get_mut(&dependency_idx)
        .unwrap()
        .insert("*".into());
    }

    dependency_idx
  }

  pub fn has_edge(&mut self, from_idx: &NodeId, to_idx: &NodeId) -> bool {
    self.graph.contains_edge(
      self.node_id_to_node_index[from_idx],
      self.node_id_to_node_index[to_idx],
    )
  }

  pub fn add_edge(&mut self, from_idx: &NodeId, to_idx: &NodeId) {
    self.graph.add_edge(
      self.node_id_to_node_index[from_idx],
      self.node_id_to_node_index[to_idx],
    )
  }

  pub fn get_outgoing_dependencies(&self, asset_node_id: &NodeId) -> Vec<NodeId> {
    self
      .graph
      .neighbors_directed(
        self.node_id_to_node_index[asset_node_id],
        Direction::Outgoing,
      )
      .filter_map(|node_index| self.graph.node_weight(node_index).map(|n| *n))
      .collect()
  }

  pub fn resolve_dependency_asset(&self, dep_node_id: &NodeId) -> Option<&NodeId> {
    if let Some(resolved) = self
      .graph
      .edges_directed(self.node_id_to_node_index[dep_node_id], Direction::Outgoing)
      .next()
    {
      return self.graph.node_weight(resolved.target());
    }

    None
  }

  pub fn get_requested_symbols(&self, node_id: &NodeId) -> &HashSet<String> {
    // TODO: Should probably do error handling here...
    self.requested_symbols.get(node_id).unwrap()
  }

  pub fn get_mut_requested_symbols(&mut self, node_id: &NodeId) -> &mut HashSet<String> {
    // TODO: Should probably do error handling here...
    self.requested_symbols.get_mut(node_id).unwrap()
  }
  pub fn get_mut_dependency_state(&mut self, node_id: &NodeId) -> &mut DependencyState {
    // TODO: Should probably do error handling here...
    self.dependency_states.get_mut(node_id).unwrap()
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

  fn assert_requested_symbols(graph: &AssetGraph, idx: NodeId, expected: Vec<&str>) {
    assert_eq!(
      graph.get_dependency_node(&idx).unwrap().requested_symbols,
      expected
        .into_iter()
        .map(|s| s.into())
        .collect::<HashSet<String>>()
    );
  }

  fn add_asset(
    graph: &mut AssetGraph,
    parent_node: NodeId,
    symbols: Vec<TestSymbol>,
    file_path: &str,
  ) -> NodeId {
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
    parent_node: NodeId,
    symbols: Vec<TestSymbol>,
  ) -> NodeId {
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

    propagate_requested_symbols(
      &mut graph,
      index_asset_node,
      entry_dep_node,
      &mut |dependency_node_index, _| {
        requested.insert(dependency_node_index);
      },
    );

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
    propagate_requested_symbols(&mut graph, entry_asset_node, entry_dep_node, &mut |_, _| {});

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

    propagate_requested_symbols(
      &mut graph,
      library_asset_node,
      library_dep_node,
      &mut |dependency_node_index, _| {
        requested_deps.push(dependency_node_index);
      },
    );

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
    propagate_requested_symbols(&mut graph, entry_asset_node, entry_dep_node, &mut |_, _| {});

    // library.js re-exports "*" from a.js and "*" from b.js
    // only "a" is used in entry.js
    let library_asset_node = add_asset(&mut graph, library_dep_node, vec![], "library.js");
    let a_dep = add_dependency(&mut graph, library_asset_node, vec![("*", "*", true)]);
    let b_dep = add_dependency(&mut graph, library_asset_node, vec![("*", "*", true)]);

    let mut requested_deps = Vec::new();
    propagate_requested_symbols(
      &mut graph,
      library_asset_node,
      library_dep_node,
      &mut |dependency_node_index, _| {
        requested_deps.push(dependency_node_index);
      },
    );
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
    propagate_requested_symbols(&mut graph, entry_asset_node, entry_dep_node, &mut |_, _| {});

    // library.js re-exports "*" from library/index.js
    let library_entry_asset_node = add_asset(&mut graph, library_dep_node, vec![], "library.js");
    let library_reexport_dep_node =
      add_dependency(&mut graph, library_entry_asset_node, vec![("*", "*", true)]);
    propagate_requested_symbols(
      &mut graph,
      library_entry_asset_node,
      library_dep_node,
      &mut |_, _| {},
    );

    // library/index.js re-exports "a" from a.js
    let library_asset_node = add_asset(
      &mut graph,
      library_reexport_dep_node,
      vec![("a", "a", true)],
      "library/index.js",
    );
    let a_dep = add_dependency(&mut graph, library_asset_node, vec![("a", "a", true)]);
    propagate_requested_symbols(
      &mut graph,
      library_entry_asset_node,
      library_dep_node,
      &mut |_, _| {},
    );

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
    propagate_requested_symbols(&mut graph, entry_asset_node, entry_dep_node, &mut |_, _| {});

    // library.js re-exports "b" from b.js renamed as "a"
    let library_asset_node = add_asset(
      &mut graph,
      library_dep_node,
      vec![("b", "a", true)],
      "library.js",
    );
    let b_dep = add_dependency(&mut graph, library_asset_node, vec![("b", "b", true)]);
    propagate_requested_symbols(
      &mut graph,
      library_asset_node,
      library_dep_node,
      &mut |_, _| {},
    );

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
    propagate_requested_symbols(&mut graph, entry_asset_node, entry_dep_node, &mut |_, _| {});

    // library.js re-exports "*" from stuff.js renamed as "a""
    // export * as a from './stuff.js'
    let library_asset_node = add_asset(
      &mut graph,
      library_dep_node,
      vec![("a", "a", true)],
      "library.js",
    );
    let stuff_dep = add_dependency(&mut graph, library_asset_node, vec![("a", "*", true)]);
    propagate_requested_symbols(
      &mut graph,
      library_asset_node,
      library_dep_node,
      &mut |_, _| {},
    );

    // "a" should be marked as requested on the library dep
    assert_requested_symbols(&graph, library_dep_node, vec!["a"]);
    // "*" should be marked as requested on the stuff dep
    assert_requested_symbols(&graph, stuff_dep, vec!["*"]);
  }
}
