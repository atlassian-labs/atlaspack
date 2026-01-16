mod serialize;

use petgraph::graph::NodeIndex;
use petgraph::visit::Dfs;
use serde::Deserialize;
use serde::Serialize;
use tracing::field;

use std::collections::HashMap;
use std::path::PathBuf;

use crate::types::Bundle;
use crate::types::Dependency;
use crate::types::SourceLocation;
use crate::types::Target;

use serialize::deserialize_map_from_js_map;
use serialize::deserialize_set_from_array_or_map;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
  pub file_path: PathBuf,
  pub package_path: PathBuf,
  #[serde(default)]
  pub target: Option<String>,
  #[serde(default)]
  pub loc: Option<SourceLocation>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleGroup {
  pub target: Target,
  pub entry_asset_id: String,
}

// Deserialization structs for each node type

#[derive(Debug)]
pub enum BundleGraphNode {
  Asset(AssetNode),
  Dependency(DependencyNode),
  EntrySpecifier(EntrySpecifierNode),
  EntryFile(EntryFileNode),
  Root(RootNode),
  BundleGroup(BundleGroupNode),
  Bundle(BundleNode),
}

impl BundleGraphNode {
  pub fn id(&self) -> &str {
    match self {
      BundleGraphNode::Asset(node) => &node.id,
      BundleGraphNode::Dependency(node) => &node.id,
      BundleGraphNode::EntrySpecifier(node) => &node.id,
      BundleGraphNode::EntryFile(node) => &node.id,
      BundleGraphNode::Root(node) => &node.id,
      BundleGraphNode::BundleGroup(node) => &node.id,
      BundleGraphNode::Bundle(node) => &node.id,
    }
  }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetNode {
  pub id: String,
  #[serde(rename = "type")]
  pub node_type: String,
  pub value: Asset,
  #[serde(deserialize_with = "deserialize_set_from_array_or_map")]
  pub used_symbols: HashSet<String>,
  #[serde(default)]
  pub has_deferred: Option<bool>,
  pub used_symbols_down_dirty: bool,
  pub used_symbols_up_dirty: bool,
  #[serde(default)]
  pub requested: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyNode {
  pub id: String,
  #[serde(rename = "type")]
  pub node_type: String,
  pub value: Dependency,
  #[serde(default)]
  pub complete: Option<bool>,
  #[serde(default)]
  pub corresponding_request: Option<String>,
  pub deferred: bool,
  #[serde(default)]
  pub has_deferred: Option<bool>,
  #[serde(deserialize_with = "deserialize_set_from_array_or_map")]
  pub used_symbols_down: HashSet<String>,
  #[serde(deserialize_with = "deserialize_map_from_js_map")]
  pub used_symbols_up: HashMap<String, Option<UsedSymbolResolution>>,
  pub used_symbols_down_dirty: bool,
  pub used_symbols_up_dirty_down: bool,
  pub used_symbols_up_dirty_up: bool,
  pub excluded: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsedSymbolResolution {
  pub asset: String,
  pub symbol: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntrySpecifierNode {
  pub id: String,
  #[serde(rename = "type")]
  pub node_type: String,
  pub value: PathBuf,
  #[serde(default)]
  pub corresponding_request: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryFileNode {
  pub id: String,
  #[serde(rename = "type")]
  pub node_type: String,
  pub value: Entry,
  #[serde(default)]
  pub corresponding_request: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RootNode {
  pub id: String,
  #[serde(rename = "type")]
  pub node_type: String,
  pub value: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleGroupNode {
  pub id: String,
  #[serde(rename = "type")]
  pub node_type: String,
  pub value: BundleGroup,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleNode {
  pub id: String,
  #[serde(rename = "type")]
  pub node_type: String,
  pub value: Bundle,
}

use std::collections::HashSet;

use petgraph::prelude::StableDiGraph;

use crate::types::Asset;

pub type BundleGraphNodeId = String;

pub struct BundleGraph {
  pub graph: StableDiGraph<BundleGraphNode, ()>,
  nodes: HashMap<u32, NodeIndex>,
}

impl BundleGraph {
  pub fn new() -> Self {
    Self {
      graph: StableDiGraph::new(),
      nodes: HashMap::new(),
    }
  }

  pub fn add_node(&mut self, index: u32, node: BundleGraphNode) {
    let node_index = self.graph.add_node(node);
    self.nodes.insert(index, node_index);
  }

  pub fn add_edge(&mut self, source: u32, target: u32) {
    let from = self.nodes.get(&source).unwrap();
    let to = self.nodes.get(&target).unwrap();
    self.graph.add_edge(*from, *to, ());
  }

  pub fn traverse_bundles(&self, visit: impl Fn(&Bundle)) {
    let root = self.nodes.get(&0).unwrap();
    let mut dfs = Dfs::new(&self.graph, *root);
    while let Some(node) = dfs.next(&self.graph) {
      let node = self.graph.node_weight(node).unwrap();
      if let BundleGraphNode::Bundle(node) = node {
        visit(&node.value);
      }
    }
  }
}

impl Default for BundleGraph {
  fn default() -> Self {
    Self::new()
  }
}
