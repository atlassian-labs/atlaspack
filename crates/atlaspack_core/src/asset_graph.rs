use std::{collections::HashSet, sync::Arc};

use petgraph::{
  graph::{DiGraph, NodeIndex},
  visit::EdgeRef,
  Direction,
};
use serde::Serialize;

use crate::types::{Asset, Dependency};

#[derive(Clone, Debug)]
pub struct AssetGraph {
  graph: DiGraph<AssetGraphNode, AssetGraphEdge>,
  pub assets: Vec<AssetNode>,
  pub dependencies: Vec<DependencyNode>,
}

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
pub enum AssetGraphNode {
  Root,
  Entry,
  Asset(usize),
  Dependency(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssetGraphEdge {}

#[derive(Clone, Debug, PartialEq)]
pub enum DependencyState {
  New,
  Deferred,
  Excluded,
  Resolved,
}

impl PartialEq for AssetGraph {
  fn eq(&self, other: &Self) -> bool {
    let nodes = self.graph.raw_nodes().iter().map(|n| &n.weight);
    let other_nodes = other.graph.raw_nodes().iter().map(|n| &n.weight);

    let edges = self
      .graph
      .raw_edges()
      .iter()
      .map(|e| (e.source(), e.target(), &e.weight));

    let other_edges = other
      .graph
      .raw_edges()
      .iter()
      .map(|e| (e.source(), e.target(), &e.weight));

    nodes.eq(other_nodes)
      && edges.eq(other_edges)
      && self.assets == other.assets
      && self.dependencies == other.dependencies
  }
}

impl Default for AssetGraph {
  fn default() -> Self {
    Self::new()
  }
}

impl AssetGraph {
  pub fn new() -> Self {
    let mut graph = DiGraph::new();

    graph.add_node(AssetGraphNode::Root);

    AssetGraph {
      graph,
      assets: Vec::new(),
      dependencies: Vec::new(),
    }
  }

  pub fn edges(&self) -> Vec<u32> {
    let raw_edges = self.graph.raw_edges();
    let mut edges = Vec::with_capacity(raw_edges.len() * 2);

    for edge in raw_edges {
      edges.push(edge.source().index() as u32);
      edges.push(edge.target().index() as u32);
    }

    edges
  }

  pub fn nodes(&self) -> impl Iterator<Item = &AssetGraphNode> {
    let nodes = self.graph.node_weights();

    nodes
  }

  pub fn add_asset(&mut self, parent_idx: NodeIndex, asset: Asset) -> NodeIndex {
    let idx = self.assets.len();

    self.assets.push(AssetNode {
      asset,
      requested_symbols: HashSet::default(),
    });

    let asset_idx = self.graph.add_node(AssetGraphNode::Asset(idx));

    self
      .graph
      .add_edge(parent_idx, asset_idx, AssetGraphEdge {});

    asset_idx
  }

  pub fn add_entry_dependency(&mut self, dependency: Dependency) -> NodeIndex {
    // The root node index will always be 0
    let root_node_index = NodeIndex::new(0);

    let is_library = dependency.env.is_library;
    let node_index = self.add_dependency(root_node_index, dependency);

    if is_library {
      if let Some(dependency_index) = &self.dependency_index(node_index) {
        self.dependencies[*dependency_index]
          .requested_symbols
          .insert("*".into());
      }
    }

    node_index
  }

  pub fn add_dependency(&mut self, parent_idx: NodeIndex, dependency: Dependency) -> NodeIndex {
    let idx = self.dependencies.len();

    self.dependencies.push(DependencyNode {
      dependency: Arc::new(dependency),
      requested_symbols: HashSet::default(),
      state: DependencyState::New,
    });

    let dependency_idx = self.graph.add_node(AssetGraphNode::Dependency(idx));

    self
      .graph
      .add_edge(parent_idx, dependency_idx, AssetGraphEdge {});

    dependency_idx
  }

  pub fn add_edge(&mut self, parent_idx: &NodeIndex, child_idx: &NodeIndex) {
    self
      .graph
      .add_edge(*parent_idx, *child_idx, AssetGraphEdge {});
  }

  pub fn dependency_index(&self, node_index: NodeIndex) -> Option<usize> {
    match self.graph.node_weight(node_index).unwrap() {
      AssetGraphNode::Dependency(idx) => Some(*idx),
      _ => None,
    }
  }

  pub fn asset_index(&self, node_index: NodeIndex) -> Option<usize> {
    match self.graph.node_weight(node_index).unwrap() {
      AssetGraphNode::Asset(idx) => Some(*idx),
      _ => None,
    }
  }

  /// Propagates the requested symbols from an incoming dependency to an asset,
  /// and forwards those symbols to re-exported dependencies if needed.
  /// This may result in assets becoming un-deferred and transformed if they
  /// now have requested symbols.
  pub fn propagate_requested_symbols<F: FnMut(NodeIndex, Arc<Dependency>)>(
    &mut self,
    asset_node: NodeIndex,
    incoming_dep_node: NodeIndex,
    on_undeferred: &mut F,
  ) {
    let DependencyNode {
      requested_symbols, ..
    } = &self.dependencies[self.dependency_index(incoming_dep_node).unwrap()];

    let asset_index = self.asset_index(asset_node).unwrap();
    let AssetNode {
      asset,
      requested_symbols: asset_requested_symbols,
    } = &mut self.assets[asset_index];

    let mut re_exports = HashSet::<String>::default();
    let mut wildcards = HashSet::<String>::default();
    let star = String::from("*");

    if requested_symbols.contains(&star) {
      // If the requested symbols includes the "*" namespace, we need to include all of the asset's
      // exported symbols.
      if let Some(symbols) = &asset.symbols {
        for sym in symbols {
          if asset_requested_symbols.insert(sym.exported.clone()) && sym.is_weak {
            // Propagate re-exported symbol to dependency.
            re_exports.insert(sym.local.clone());
          }
        }
      }

      // Propagate to all export * wildcard dependencies.
      wildcards.insert(star);
    } else {
      // Otherwise, add each of the requested symbols to the asset.
      for sym in requested_symbols.iter() {
        if asset_requested_symbols.insert(sym.clone()) {
          if let Some(asset_symbol) = asset
            .symbols
            .as_ref()
            .and_then(|symbols| symbols.iter().find(|s| s.exported == *sym))
          {
            if asset_symbol.is_weak {
              // Propagate re-exported symbol to dependency.
              re_exports.insert(asset_symbol.local.clone());
            }
          } else {
            // If symbol wasn't found in the asset or a named re-export.
            // This means the symbol is in one of the export * wildcards, but we don't know
            // which one yet, so we propagate it to _all_ wildcard dependencies.
            wildcards.insert(sym.clone());
          }
        }
      }
    }

    let deps: Vec<_> = self
      .graph
      .neighbors_directed(asset_node, Direction::Outgoing)
      .collect();
    for dep_node in deps {
      let dep_index = self.dependency_index(dep_node).unwrap();
      let DependencyNode {
        dependency,
        requested_symbols,
        state,
      } = &mut self.dependencies[dep_index];

      let mut updated = false;
      if let Some(symbols) = &dependency.symbols {
        for sym in symbols {
          if sym.is_weak {
            // This is a re-export. If it is a wildcard, add all unmatched symbols
            // to this dependency, otherwise attempt to match a named re-export.
            if sym.local == "*" {
              for wildcard in &wildcards {
                if requested_symbols.insert(wildcard.clone()) {
                  updated = true;
                }
              }
            } else if re_exports.contains(&sym.local)
              && requested_symbols.insert(sym.exported.clone())
            {
              updated = true;
            }
          } else if requested_symbols.insert(sym.exported.clone()) {
            // This is a normal import. Add the requested symbol.
            updated = true;
          }
        }
      }

      // If the dependency was updated, propagate to the target asset if there is one,
      // or un-defer this dependency so we transform the requested asset.
      // We must always resolve new dependencies to determine whether they have side effects.
      if updated || *state == DependencyState::New {
        if let Some(resolved) = self
          .graph
          .edges_directed(dep_node, Direction::Outgoing)
          .next()
        {
          // Avoid infintite loops for self references
          if resolved.target() != asset_node {
            self.propagate_requested_symbols(resolved.target(), dep_node, on_undeferred);
          }
        } else {
          on_undeferred(dep_node, Arc::clone(dependency));
        }
      }
    }
  }

  pub fn serialize_nodes(&self, max_str_len: usize) -> serde_json::Result<Vec<String>> {
    let mut nodes: Vec<String> = Vec::new();
    let mut curr_node = String::default();

    for node in self.nodes() {
      let serialized_node = match node {
        AssetGraphNode::Root => SerializedAssetGraphNode::Root,
        AssetGraphNode::Entry => SerializedAssetGraphNode::Entry,
        AssetGraphNode::Asset(idx) => {
          let asset = self.assets[*idx].asset.clone();

          SerializedAssetGraphNode::Asset { value: asset }
        }
        AssetGraphNode::Dependency(idx) => {
          let dependency = self.dependencies[*idx].dependency.clone();
          SerializedAssetGraphNode::Dependency {
            value: SerializedDependency {
              id: dependency.id(),
              dependency: dependency.as_ref().clone(),
            },
            has_deferred: self.dependencies[*idx].state == DependencyState::Deferred,
          }
        }
      };

      let str = serde_json::to_string(&serialized_node)?;
      if curr_node.len() + str.len() < (max_str_len - 3) {
        if !curr_node.is_empty() {
          curr_node.push(',');
        }
        curr_node.push_str(&str);
      } else {
        // Add the existing node now as it has reached the max JavaScript string size
        nodes.push(format!("[{curr_node}]"));
        curr_node = str;
      }
    }

    // Add the current node if it did not overflow in size
    if curr_node.len() < (max_str_len - 3) {
      nodes.push(format!("[{curr_node}]"));
    }

    Ok(nodes)
  }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedDependency {
  id: String,
  dependency: Dependency,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum SerializedAssetGraphNode {
  Root,
  Entry,
  Asset {
    value: Asset,
  },
  Dependency {
    value: SerializedDependency,
    has_deferred: bool,
  },
}

impl std::hash::Hash for AssetGraph {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    for node in self.graph.node_weights() {
      std::mem::discriminant(node).hash(state);
      match node {
        AssetGraphNode::Asset(idx) => self.assets[*idx].asset.id.hash(state),
        AssetGraphNode::Dependency(idx) => self.dependencies[*idx].dependency.id().hash(state),
        _ => {}
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use serde_json::{json, Value};

  use crate::types::{Symbol, Target};

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

  fn assert_requested_symbols(graph: &AssetGraph, node_index: NodeIndex, expected: Vec<&str>) {
    assert_eq!(
      graph.dependencies[graph.dependency_index(node_index).unwrap()].requested_symbols,
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
    graph.add_asset(parent_node, index_asset)
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
    graph.add_dependency(parent_node, dep)
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
    graph.propagate_requested_symbols(
      index_asset_node,
      entry_dep_node,
      &mut |dependency_node_index, _dependency| {
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
    graph.propagate_requested_symbols(entry_asset_node, entry_dep_node, &mut |_, _| {});

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
    graph.propagate_requested_symbols(
      library_asset_node,
      library_dep_node,
      &mut |dependency_node_index, _dependency| {
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
    graph.propagate_requested_symbols(entry_asset_node, entry_dep_node, &mut |_, _| {});

    // library.js re-exports "*" from a.js and "*" from b.js
    // only "a" is used in entry.js
    let library_asset_node = add_asset(&mut graph, library_dep_node, vec![], "library.js");
    let a_dep = add_dependency(&mut graph, library_asset_node, vec![("*", "*", true)]);
    let b_dep = add_dependency(&mut graph, library_asset_node, vec![("*", "*", true)]);

    let mut requested_deps = Vec::new();
    graph.propagate_requested_symbols(
      library_asset_node,
      library_dep_node,
      &mut |dependency_node_index, _dependency| {
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
    graph.propagate_requested_symbols(entry_asset_node, entry_dep_node, &mut |_, _| {});

    // library.js re-exports "*" from library/index.js
    let library_entry_asset_node = add_asset(&mut graph, library_dep_node, vec![], "library.js");
    let library_reexport_dep_node =
      add_dependency(&mut graph, library_entry_asset_node, vec![("*", "*", true)]);
    graph.propagate_requested_symbols(library_entry_asset_node, library_dep_node, &mut |_, _| {});

    // library/index.js re-exports "a" from a.js
    let library_asset_node = add_asset(
      &mut graph,
      library_reexport_dep_node,
      vec![("a", "a", true)],
      "library/index.js",
    );
    let a_dep = add_dependency(&mut graph, library_asset_node, vec![("a", "a", true)]);
    graph.propagate_requested_symbols(library_entry_asset_node, library_dep_node, &mut |_, _| {});

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
    graph.propagate_requested_symbols(entry_asset_node, entry_dep_node, &mut |_, _| {});

    // library.js re-exports "b" from b.js renamed as "a"
    let library_asset_node = add_asset(
      &mut graph,
      library_dep_node,
      vec![("b", "a", true)],
      "library.js",
    );
    let b_dep = add_dependency(&mut graph, library_asset_node, vec![("b", "b", true)]);
    graph.propagate_requested_symbols(library_asset_node, library_dep_node, &mut |_, _| {});

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
    graph.propagate_requested_symbols(entry_asset_node, entry_dep_node, &mut |_, _| {});

    // library.js re-exports "*" from stuff.js renamed as "a""
    // export * as a from './stuff.js'
    let library_asset_node = add_asset(
      &mut graph,
      library_dep_node,
      vec![("a", "a", true)],
      "library.js",
    );
    let stuff_dep = add_dependency(&mut graph, library_asset_node, vec![("a", "*", true)]);
    graph.propagate_requested_symbols(library_asset_node, library_dep_node, &mut |_, _| {});

    // "a" should be marked as requested on the library dep
    assert_requested_symbols(&graph, library_dep_node, vec!["a"]);
    // "*" should be marked as requested on the stuff dep
    assert_requested_symbols(&graph, stuff_dep, vec!["*"]);
  }

  #[test]
  fn serialize_nodes_handles_max_size() -> anyhow::Result<()> {
    let mut graph = AssetGraph::new();

    let entry = graph.add_entry_dependency(Dependency {
      specifier: String::from("entry"),
      ..Dependency::default()
    });

    let entry_asset = graph.add_asset(
      entry,
      Asset {
        file_path: PathBuf::from("entry"),
        ..Asset::default()
      },
    );

    for i in 1..100 {
      graph.add_dependency(
        entry_asset,
        Dependency {
          specifier: format!("dependency-{}", i),
          ..Dependency::default()
        },
      );
    }

    let max_str_len = 10000;
    let nodes = graph.serialize_nodes(max_str_len)?;

    assert_eq!(nodes.len(), 7);

    // Assert each string is less than the max size
    for node in nodes.iter() {
      assert!(node.len() < max_str_len);
    }

    // Assert all the nodes are included and in the correct order
    let first_entry = serde_json::from_str::<Value>(&nodes[0])?;
    let first_entry = first_entry.as_array().unwrap();

    assert_eq!(get_type(&first_entry[0]), json!("root"));
    assert_eq!(get_dependency(&first_entry[1]), Some(json!("entry")));
    assert_eq!(get_asset(&first_entry[2]), Some(json!("entry")));

    for i in 1..first_entry.len() - 2 {
      assert_eq!(
        get_dependency(&first_entry[i + 2]),
        Some(json!(format!("dependency-{}", i)))
      );
    }

    let mut specifier = first_entry.len() - 2;
    for node in nodes[1..].iter() {
      let entry = serde_json::from_str::<Value>(node)?;
      let entry = entry.as_array().unwrap();

      for value in entry {
        assert_eq!(
          get_dependency(value),
          Some(json!(format!("dependency-{}", specifier)))
        );

        specifier += 1;
      }
    }

    Ok(())
  }

  fn get_type(node: &Value) -> Value {
    node.get("type").unwrap().to_owned()
  }

  fn get_dependency(value: &Value) -> Option<Value> {
    assert_eq!(get_type(value), json!("dependency"));

    value
      .get("value")
      .unwrap()
      .get("dependency")
      .unwrap()
      .get("specifier")
      .map(|s| s.to_owned())
  }

  fn get_asset(value: &Value) -> Option<Value> {
    assert_eq!(get_type(value), json!("asset"));

    value
      .get("value")
      .unwrap()
      .get("filePath")
      .map(|s| s.to_owned())
  }
}
