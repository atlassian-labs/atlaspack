use std::collections::{HashMap, HashSet};

use atlaspack_core::{
  asset_graph::{AssetGraph, AssetGraphNode},
  types::{Asset, Dependency},
};

/// Configuration knobs for the ideal graph build/analysis.
///
/// This is expected to grow as we implement more of the research doc.
#[derive(Debug, Clone, Default)]
pub struct IdealGraphBuildOptions {
  /// When true, the builder will collect additional debugging metadata.
  pub collect_debug: bool,
}

/// Summary stats from building an [`IdealGraph`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IdealGraphBuildStats {
  pub assets: usize,
  pub dependencies: usize,
}

/// A stable identifier for nodes in the ideal graph.
///
/// We keep this separate from petgraph indices so we can change the underlying
/// graph representation without rewriting all consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdealNodeId(pub u32);

/// Node payload.
#[derive(Debug, Clone)]
pub enum IdealNode {
  Asset { id: String },
  Dependency { id: String },
}

/// Intermediate graph representation used by the bundler algorithm.
///
/// Initial implementation is intentionally minimal: just the subset we need
/// to get started and write tests.
#[derive(Debug, Default, Clone)]
pub struct IdealGraph {
  pub nodes: Vec<IdealNode>,
  pub edges: Vec<(IdealNodeId, IdealNodeId)>,

  /// Fast lookup for AssetGraph content keys.
  pub index_by_content_key: HashMap<String, IdealNodeId>,
}

impl IdealGraph {
  pub fn from_asset_graph(
    asset_graph: &AssetGraph,
    _options: &IdealGraphBuildOptions,
  ) -> anyhow::Result<(Self, IdealGraphBuildStats)> {
    let mut g = IdealGraph::default();
    let mut stats = IdealGraphBuildStats::default();

    // 1) Create nodes for assets/dependencies.
    for node in asset_graph.nodes() {
      match node {
        AssetGraphNode::Asset(asset) => {
          let id = g.push_node(IdealNode::Asset {
            id: asset.id.clone(),
          });
          g.index_by_content_key.insert(asset.id.clone(), id);
          stats.assets += 1;
        }
        AssetGraphNode::Dependency(dep) => {
          let id = g.push_node(IdealNode::Dependency { id: dep.id.clone() });
          g.index_by_content_key.insert(dep.id.clone(), id);
          stats.dependencies += 1;
        }
        _ => {}
      }
    }

    // 2) Create edges by walking AssetGraph connectivity.
    //
    // Important: `AssetGraph` uses its own `NodeId` (currently `usize`) rather than petgraph's
    // `NodeIndex`, so we only use the public `AssetGraph` APIs.
    let mut node_ids_by_key: Vec<(String, usize)> = Vec::new();

    for node in asset_graph.nodes() {
      match node {
        AssetGraphNode::Asset(a) => {
          if let Some(node_id) = asset_graph.get_node_id_by_content_key(&a.id) {
            node_ids_by_key.push((a.id.clone(), *node_id));
          }
        }
        AssetGraphNode::Dependency(d) => {
          if let Some(node_id) = asset_graph.get_node_id_by_content_key(&d.id) {
            node_ids_by_key.push((d.id.clone(), *node_id));
          }
        }
        _ => {}
      }
    }

    for (from_key, from_node_id) in node_ids_by_key {
      let Some(from) = g.index_by_content_key.get(&from_key).copied() else {
        continue;
      };

      for to in asset_graph.get_outgoing_neighbors(&from_node_id) {
        let Some(to_node) = asset_graph.get_node(&to) else {
          continue;
        };

        let to_key: Option<&str> = match to_node {
          AssetGraphNode::Asset(a) => Some(a.id.as_str()),
          AssetGraphNode::Dependency(d) => Some(d.id.as_str()),
          _ => None,
        };

        let Some(to_key) = to_key else {
          continue;
        };

        if let Some(to_id) = g.index_by_content_key.get(to_key).copied() {
          g.edges.push((from, to_id));
        }
      }
    }

    Ok((g, stats))
  }

  pub fn push_node(&mut self, node: IdealNode) -> IdealNodeId {
    let id = IdealNodeId(u32::try_from(self.nodes.len()).expect("IdealGraph node id overflow"));
    self.nodes.push(node);
    id
  }

  pub fn node(&self, id: IdealNodeId) -> Option<&IdealNode> {
    self.nodes.get(id.0 as usize)
  }

  pub fn outgoing(&self, from: IdealNodeId) -> impl Iterator<Item = IdealNodeId> + '_ {
    self
      .edges
      .iter()
      .filter_map(move |(a, b)| (*a == from).then_some(*b))
  }

  /// Returns all node ids reachable from `start` including `start`.
  pub fn reachable(&self, start: IdealNodeId) -> HashSet<IdealNodeId> {
    let mut visited = HashSet::new();
    let mut stack = vec![start];

    while let Some(n) = stack.pop() {
      if !visited.insert(n) {
        continue;
      }

      for out in self.outgoing(n) {
        stack.push(out);
      }
    }

    visited
  }
}

// These are used by future phases, but keeping the imports here helps avoid churn
// when implementing dominators/placement logic.
#[allow(unused_imports)]
fn _type_anchors(_a: &Asset, _d: &Dependency) {}

#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use atlaspack_core::{
    asset_graph::AssetGraph,
    types::{Asset, Dependency, Environment, FileType, Target},
  };
  use pretty_assertions::assert_eq;

  use super::*;

  #[test]
  fn builds_nodes_and_edges_from_asset_graph() {
    let mut asset_graph = AssetGraph::new();

    // entry dep -> entry asset -> dep2 -> asset2
    let target = Target::default();
    let entry_dep = Dependency::entry("entry.js".to_string(), target);
    let entry_dep_node = asset_graph.add_entry_dependency(entry_dep, false);

    let entry_asset = Arc::new(Asset {
      id: "entry_asset".into(),
      file_path: "entry.js".into(),
      file_type: FileType::Js,
      env: Arc::new(Environment::default()),
      ..Asset::default()
    });
    let entry_asset_node = asset_graph.add_asset(entry_asset, false);
    asset_graph.add_edge(&entry_dep_node, &entry_asset_node);

    let dep2 = atlaspack_core::types::DependencyBuilder::default()
      .specifier("./asset2.js".to_string())
      .specifier_type(atlaspack_core::types::SpecifierType::Esm)
      .env(Arc::new(Environment::default()))
      .priority(atlaspack_core::types::Priority::default())
      .build();
    let dep2_id = dep2.id.clone();
    let dep2_node = asset_graph.add_dependency(dep2, false);
    asset_graph.add_edge(&entry_asset_node, &dep2_node);

    let asset2 = Arc::new(Asset {
      id: "asset2".into(),
      file_path: "asset2.js".into(),
      file_type: FileType::Js,
      env: Arc::new(Environment::default()),
      ..Asset::default()
    });
    let asset2_node = asset_graph.add_asset(asset2, false);
    asset_graph.add_edge(&dep2_node, &asset2_node);

    let (g, stats) =
      IdealGraph::from_asset_graph(&asset_graph, &IdealGraphBuildOptions::default()).unwrap();

    assert_eq!(stats.assets, 2);
    assert_eq!(stats.dependencies, 2);

    // Ensure content keys were indexed.
    assert!(g.index_by_content_key.contains_key("entry_asset"));
    assert!(g.index_by_content_key.contains_key(&dep2_id));

    // Ensure reachability works.
    let start = g.index_by_content_key["entry_asset"];
    let reachable = g.reachable(start);
    assert!(reachable.len() >= 3);

    // Sanity check we have at least one edge.
    assert!(!g.edges.is_empty());

    // Avoid unused warnings for nodes we created.
    let _ = (entry_asset_node, asset2_node);
  }
}
