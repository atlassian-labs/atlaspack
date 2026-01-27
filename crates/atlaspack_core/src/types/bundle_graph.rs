use std::{
  collections::{HashMap, HashSet},
  path::PathBuf,
};

use serde::{Deserialize, Serialize};

use crate::types::{Asset, Bundle, Dependency, SourceLocation, Target};

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

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum BundleGraphNode {
  #[serde(rename = "asset")]
  Asset(AssetNode),
  #[serde(rename = "dependency")]
  Dependency(DependencyNode),
  #[serde(rename = "entry_specifier")]
  EntrySpecifier(EntrySpecifierNode),
  #[serde(rename = "entry_file")]
  EntryFile(EntryFileNode),
  #[serde(rename = "root")]
  Root(RootNode),
  #[serde(rename = "bundle_group")]
  BundleGroup(BundleGroupNode),
  #[serde(rename = "bundle")]
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
  pub value: Asset,
  // TODO - this isn't used by the dev packager, so we'll skip for now to avoid deser complexity
  #[serde(skip)]
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
  pub value: Dependency,
  #[serde(default)]
  pub complete: Option<bool>,
  #[serde(default)]
  pub corresponding_request: Option<String>,
  pub deferred: bool,
  #[serde(default)]
  pub has_deferred: Option<bool>,
  // TODO - this isn't used by the dev packager, so we'll skip for now to avoid deser complexity
  #[serde(skip)]
  pub used_symbols_down: HashSet<String>,
  #[serde(skip)]
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
  pub value: PathBuf,
  #[serde(default)]
  pub corresponding_request: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryFileNode {
  pub id: String,
  pub value: Entry,
  #[serde(default)]
  pub corresponding_request: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RootNode {
  pub id: String,
  pub value: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleGroupNode {
  pub id: String,
  pub value: BundleGroup,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleNode {
  pub id: String,
  pub value: Bundle,
}

// TODO make this a proper enum matching BundleGraph.ts
pub type BundleGraphEdgeType = u8;
