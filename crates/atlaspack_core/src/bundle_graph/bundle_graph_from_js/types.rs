use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::types::{Asset, Bundle, Dependency, Target};

/// Edge types in the JS bundle graph.
///
/// These numeric values must match `packages/core/core/src/BundleGraph.ts`.
#[derive(Clone, Copy, Debug, Deserialize_repr, Eq, Hash, PartialEq, Serialize_repr)]
#[repr(u8)]
pub enum BundleGraphEdgeType {
  Null = 1,
  Contains = 2,
  Bundle = 3,
  References = 4,
  /// In JS, `internal_async` and `conditional` both use 5.
  InternalAsync = 5,
}

impl From<u8> for BundleGraphEdgeType {
  fn from(value: u8) -> Self {
    match value {
      1 => BundleGraphEdgeType::Null,
      2 => BundleGraphEdgeType::Contains,
      3 => BundleGraphEdgeType::Bundle,
      4 => BundleGraphEdgeType::References,
      5 => BundleGraphEdgeType::InternalAsync,
      _ => BundleGraphEdgeType::Null,
    }
  }
}

#[derive(Clone, Debug, Deserialize, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RootNode {
  pub id: String,
  #[serde(rename = "type")]
  pub node_type: String,
  pub value: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntrySpecifierNode {
  pub id: String,
  #[serde(rename = "type")]
  pub node_type: String,
  pub value: String,
  pub corresponding_request: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryFileNode {
  pub id: String,
  #[serde(rename = "type")]
  pub node_type: String,
  pub value: serde_json::Value,
  pub corresponding_request: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetNode {
  pub id: String,
  #[serde(rename = "type")]
  pub node_type: String,
  pub value: Asset,

  #[serde(default)]
  pub used_symbols: serde_json::Value,
  #[serde(default)]
  pub has_deferred: Option<bool>,
  #[serde(default)]
  pub used_symbols_down_dirty: bool,
  #[serde(default)]
  pub used_symbols_up_dirty: bool,
  #[serde(default)]
  pub requested: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Hash, PartialEq, Serialize)]
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
  #[serde(default)]
  pub deferred: bool,
  #[serde(default)]
  pub has_deferred: Option<bool>,
  #[serde(default)]
  pub used_symbols_down: serde_json::Value,
  #[serde(default)]
  pub used_symbols_up: serde_json::Value,
  #[serde(default)]
  pub used_symbols_down_dirty: bool,
  #[serde(default)]
  pub used_symbols_up_dirty_down: bool,
  #[serde(default)]
  pub used_symbols_up_dirty_up: bool,
  #[serde(default)]
  pub excluded: bool,
}

#[derive(Clone, Debug, Deserialize, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleNode {
  pub id: String,
  #[serde(rename = "type")]
  pub node_type: String,
  pub value: Bundle,
}

#[derive(Clone, Debug, Deserialize, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleGroup {
  pub target: Target,
  pub entry_asset_id: String,
}

#[derive(Clone, Debug, Deserialize, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleGroupNode {
  pub id: String,
  #[serde(rename = "type")]
  pub node_type: String,
  pub value: BundleGroup,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
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
      BundleGraphNode::Asset(n) => &n.id,
      BundleGraphNode::Dependency(n) => &n.id,
      BundleGraphNode::EntrySpecifier(n) => &n.id,
      BundleGraphNode::EntryFile(n) => &n.id,
      BundleGraphNode::Root(n) => &n.id,
      BundleGraphNode::BundleGroup(n) => &n.id,
      BundleGraphNode::Bundle(n) => &n.id,
    }
  }
}
