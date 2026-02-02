use std::{
  collections::{HashMap, HashSet},
  fmt::{Display, Formatter},
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

impl Display for BundleGraphNode {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    let type_name = match self {
      BundleGraphNode::Asset(_node) => "Asset",
      BundleGraphNode::Dependency(_node) => "Dependency",
      BundleGraphNode::EntrySpecifier(_node) => "EntrySpecifier",
      BundleGraphNode::EntryFile(_node) => "EntryFile",
      BundleGraphNode::Root(_node) => "Root",
      BundleGraphNode::BundleGroup(_node) => "BundleGroup",
      BundleGraphNode::Bundle(_node) => "Bundle",
    };
    write!(
      f,
      "{type_name}: {id}",
      type_name = type_name,
      id = self.id()
    )
  }
}

#[derive(Debug, Deserialize, Clone)]
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

#[derive(Debug, Deserialize, Clone)]
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

#[derive(Debug, Deserialize, Clone)]
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

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BundleNode {
  pub id: String,
  pub value: Bundle,
}

// This matches the edge types from JS in packages/core/core/src/BundleGraph.ts
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum BundleGraphEdgeType {
  Null = 1,
  Contains = 2,
  Bundle = 3,
  References = 4,
  InternalAsync = 5,
  Conditional = 6,
}

impl From<u8> for BundleGraphEdgeType {
  fn from(value: u8) -> Self {
    match value {
      1 => BundleGraphEdgeType::Null,
      2 => BundleGraphEdgeType::Contains,
      3 => BundleGraphEdgeType::Bundle,
      4 => BundleGraphEdgeType::References,
      5 => BundleGraphEdgeType::InternalAsync,
      6 => BundleGraphEdgeType::Conditional,
      _ => panic!("Invalid bundle graph edge type: {}", value),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  #[test]
  fn test_deserialize_root_node() {
    let json = r#"{"type": "root", "id": "root-1", "value": null}"#;
    let node: BundleGraphNode = serde_json::from_str(json).unwrap();
    assert_eq!(node.id(), "root-1");
    assert!(matches!(node, BundleGraphNode::Root(_)));
  }

  #[test]
  fn test_deserialize_entry_specifier_node() {
    let json = r#"{
      "type": "entry_specifier",
      "id": "entry-spec-1",
      "value": "/path/to/entry.js",
      "correspondingRequest": "req-123"
    }"#;
    let node: BundleGraphNode = serde_json::from_str(json).unwrap();
    assert_eq!(node.id(), "entry-spec-1");
    assert!(matches!(node, BundleGraphNode::EntrySpecifier(_)));
  }

  #[test]
  fn test_deserialize_entry_file_node() {
    let json = r#"{
      "type": "entry_file",
      "id": "entry-file-1",
      "value": {
        "filePath": "/src/index.js",
        "packagePath": "/src"
      }
    }"#;
    let node: BundleGraphNode = serde_json::from_str(json).unwrap();
    assert_eq!(node.id(), "entry-file-1");
    assert!(matches!(node, BundleGraphNode::EntryFile(_)));
  }

  #[test]
  fn test_deserialize_bundle_group_node() {
    let json = r#"{
      "type": "bundle_group",
      "id": "bg-1",
      "value": {
        "target": {
          "distDir": "/dist",
          "distEntry": "index.js",
          "env": {
            "context": "browser",
            "engines": {},
            "includeNodeModules": true,
            "outputFormat": "esmodule",
            "sourceType": "module",
            "isLibrary": false,
            "shouldOptimize": false,
            "shouldScopeHoist": false,
            "sourceMap": {},
            "unstableSingleFileOutput": false
          },
          "name": "default",
          "publicUrl": "/"
        },
        "entryAssetId": "asset-123"
      }
    }"#;
    let node: BundleGraphNode = serde_json::from_str(json).unwrap();
    assert_eq!(node.id(), "bg-1");
    assert!(matches!(node, BundleGraphNode::BundleGroup(_)));
  }

  #[test]
  fn test_deserialize_unknown_type_fails() {
    let json = r#"{"type": "unknown_type", "id": "test"}"#;
    let result: Result<BundleGraphNode, _> = serde_json::from_str(json);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("unknown variant"));
  }

  #[test]
  fn test_deserialize_missing_type_fails() {
    let json = r#"{"id": "test"}"#;
    let result: Result<BundleGraphNode, _> = serde_json::from_str(json);
    assert!(result.is_err());
  }

  #[test]
  fn test_deserialize_array_of_nodes() {
    let json = r#"[
      {"type": "root", "id": "root", "value": null},
      {"type": "entry_specifier", "id": "es-1", "value": "/index.js"}
    ]"#;
    let nodes: Vec<BundleGraphNode> = serde_json::from_str(json).unwrap();
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0].id(), "root");
    assert_eq!(nodes[1].id(), "es-1");
  }
}
