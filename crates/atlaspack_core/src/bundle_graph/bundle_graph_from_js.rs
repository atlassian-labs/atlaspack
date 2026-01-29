use anyhow::anyhow;
use petgraph::{graph::NodeIndex, prelude::StableDiGraph, visit::Dfs};
use rayon::prelude::*;

use crate::{
  bundle_graph::bundle_graph::BundleGraph,
  types::{Bundle, BundleGraphEdgeType, BundleGraphNode},
};

pub struct BundleGraphFromJs {
  graph: StableDiGraph<BundleGraphNode, BundleGraphEdgeType>,
}

impl BundleGraphFromJs {
  pub fn new(nodes: Vec<BundleGraphNode>, edges: Vec<(u32, u32, BundleGraphEdgeType)>) -> Self {
    let mut graph = StableDiGraph::new();
    for node in nodes {
      graph.add_node(node);
    }
    for edge in edges {
      graph.add_edge(
        NodeIndex::new(edge.0 as usize),
        NodeIndex::new(edge.1 as usize),
        edge.2,
      );
    }
    BundleGraphFromJs { graph }
  }

  #[tracing::instrument(level = "info", skip_all, fields(size))]
  pub fn deserialize_from_json(nodes_json: String) -> anyhow::Result<Vec<BundleGraphNode>> {
    // Parse JSON to Vec<Value> first (fast), then parallelize node deserialization
    let json_values: Vec<serde_json::Value> = serde_json::from_str(&nodes_json)
      .map_err(|e| anyhow!("Failed to parse bundle graph JSON: {}", e))?;

    // Parallelize the deserialization of individual nodes using rayon
    let nodes: Vec<BundleGraphNode> = json_values
      .into_par_iter()
      .map(|value| {
        serde_json::from_value::<BundleGraphNode>(value)
          .map_err(|e| anyhow!("Failed to deserialize node: {}", e))
      })
      .collect::<anyhow::Result<Vec<_>>>()?;
    tracing::Span::current().record("size", nodes.len());
    Ok(nodes)
  }
}

impl BundleGraph for BundleGraphFromJs {
  // Temporary code just to validate functionality
  fn get_bundles(&self) -> Vec<&Bundle> {
    if self.graph.node_count() == 0 {
      return Vec::new();
    }
    let mut bundles = Vec::new();
    let mut dfs = Dfs::new(&self.graph, NodeIndex::new(0));
    while let Some(node) = dfs.next(&self.graph) {
      let node = self.graph.node_weight(node).unwrap();
      if let BundleGraphNode::Bundle(node) = node {
        bundles.push(&node.value);
      }
    }
    bundles
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::types::{Asset, Dependency};
  use crate::types::{
    AssetNode, BundleNode, DependencyNode, Environment, FileType, Priority, RootNode,
    SpecifierType, Target,
  };
  use pretty_assertions::assert_eq;
  use std::path::PathBuf;
  use std::sync::Arc;

  fn create_test_bundle(id: &str, name: &str) -> Bundle {
    Bundle {
      id: id.to_string(),
      name: Some(name.to_string()),
      bundle_behavior: None,
      bundle_type: FileType::Js,
      entry_asset_ids: vec![],
      env: Environment::default(),
      hash_reference: String::new(),
      is_splittable: Some(true),
      main_entry_id: None,
      manual_shared_bundle: None,
      needs_stable_name: Some(false),
      pipeline: None,
      public_id: None,
      target: Target::default(),
    }
  }

  fn create_test_asset_node(id: &str) -> AssetNode {
    AssetNode {
      id: id.to_string(),
      value: Asset {
        id: id.to_string(),
        file_path: PathBuf::from(format!("{}.js", id)),
        file_type: FileType::Js,
        env: Arc::new(Environment::default()),
        ..Asset::default()
      },
      used_symbols: Default::default(),
      has_deferred: None,
      used_symbols_down_dirty: false,
      used_symbols_up_dirty: false,
      requested: None,
    }
  }

  fn create_test_dependency_node(id: &str) -> DependencyNode {
    DependencyNode {
      id: id.to_string(),
      value: Dependency {
        id: id.to_string(),
        specifier: "./test".to_string(),
        specifier_type: SpecifierType::Esm,
        priority: Priority::Sync,
        env: Arc::new(Environment::default()),
        ..Dependency::default()
      },
      complete: None,
      corresponding_request: None,
      deferred: false,
      has_deferred: None,
      used_symbols_down: Default::default(),
      used_symbols_up: Default::default(),
      used_symbols_down_dirty: false,
      used_symbols_up_dirty_down: false,
      used_symbols_up_dirty_up: false,
      excluded: false,
    }
  }

  fn create_test_bundle_node(id: &str, name: &str) -> BundleNode {
    BundleNode {
      id: id.to_string(),
      value: create_test_bundle(id, name),
    }
  }

  fn create_test_root_node() -> RootNode {
    RootNode {
      id: "root".to_string(),
      value: None,
    }
  }

  #[test]
  fn test_bundle_graph_from_js_new_empty() {
    let graph = BundleGraphFromJs::new(vec![], vec![]);
    assert!(graph.get_bundles().is_empty());
  }

  #[test]
  fn test_bundle_graph_from_js_single_bundle() {
    let nodes = vec![
      BundleGraphNode::Root(create_test_root_node()),
      BundleGraphNode::Bundle(create_test_bundle_node("bundle1", "main.js")),
    ];
    let edges = vec![(0, 1, 1u8)]; // Edge from root to bundle

    let graph = BundleGraphFromJs::new(nodes, edges);
    let bundles = graph.get_bundles();

    assert_eq!(bundles.len(), 1);
    assert_eq!(bundles[0].id, "bundle1");
    assert_eq!(bundles[0].name, Some("main.js".to_string()));
  }

  #[test]
  fn test_bundle_graph_from_js_multiple_bundles() {
    let nodes = vec![
      BundleGraphNode::Root(create_test_root_node()),
      BundleGraphNode::Bundle(create_test_bundle_node("bundle1", "main.js")),
      BundleGraphNode::Bundle(create_test_bundle_node("bundle2", "chunk-a.js")),
      BundleGraphNode::Bundle(create_test_bundle_node("bundle3", "chunk-b.js")),
    ];
    let edges = vec![
      (0, 1, 1u8), // root -> bundle1
      (1, 2, 2u8), // bundle1 -> bundle2
      (1, 3, 2u8), // bundle1 -> bundle3
    ];

    let graph = BundleGraphFromJs::new(nodes, edges);
    let bundles = graph.get_bundles();

    assert_eq!(bundles.len(), 3);
    let bundle_ids: Vec<&str> = bundles.iter().map(|b| b.id.as_str()).collect();
    assert!(bundle_ids.contains(&"bundle1"));
    assert!(bundle_ids.contains(&"bundle2"));
    assert!(bundle_ids.contains(&"bundle3"));
  }

  #[test]
  fn test_bundle_graph_from_js_mixed_node_types() {
    // Test that get_bundles only returns Bundle nodes, not other node types
    let nodes = vec![
      BundleGraphNode::Root(create_test_root_node()),
      BundleGraphNode::Asset(create_test_asset_node("asset1")),
      BundleGraphNode::Bundle(create_test_bundle_node("bundle1", "main.js")),
      BundleGraphNode::Dependency(create_test_dependency_node("dep1")),
      BundleGraphNode::Bundle(create_test_bundle_node("bundle2", "chunk.js")),
    ];
    let edges = vec![
      (0, 1, 1u8), // root -> asset
      (0, 2, 1u8), // root -> bundle1
      (2, 3, 2u8), // bundle1 -> dependency
      (3, 4, 2u8), // dependency -> bundle2
    ];

    let graph = BundleGraphFromJs::new(nodes, edges);
    let bundles = graph.get_bundles();

    // Only Bundle nodes should be returned
    assert_eq!(bundles.len(), 2);
    let bundle_ids: Vec<&str> = bundles.iter().map(|b| b.id.as_str()).collect();
    assert!(bundle_ids.contains(&"bundle1"));
    assert!(bundle_ids.contains(&"bundle2"));
  }

  #[test]
  fn test_bundle_graph_from_js_disconnected_bundles_not_visited() {
    // Bundles not reachable from root should not be returned by DFS
    let nodes = vec![
      BundleGraphNode::Root(create_test_root_node()),
      BundleGraphNode::Bundle(create_test_bundle_node("bundle1", "main.js")),
      BundleGraphNode::Bundle(create_test_bundle_node("bundle2", "orphan.js")),
    ];
    // Only edge from root to bundle1, bundle2 is disconnected
    let edges = vec![(0, 1, 1u8)];

    let graph = BundleGraphFromJs::new(nodes, edges);
    let bundles = graph.get_bundles();

    // Only bundle1 should be reachable
    assert_eq!(bundles.len(), 1);
    assert_eq!(bundles[0].id, "bundle1");
  }

  #[test]
  fn test_bundle_graph_node_id() {
    let root = BundleGraphNode::Root(create_test_root_node());
    assert_eq!(root.id(), "root");

    let bundle = BundleGraphNode::Bundle(create_test_bundle_node("b1", "test.js"));
    assert_eq!(bundle.id(), "b1");

    let asset = BundleGraphNode::Asset(create_test_asset_node("a1"));
    assert_eq!(asset.id(), "a1");

    let dep = BundleGraphNode::Dependency(create_test_dependency_node("d1"));
    assert_eq!(dep.id(), "d1");
  }

  #[test]
  fn test_deserialize_from_json_empty_array() {
    let json = "[]".to_string();
    let nodes = BundleGraphFromJs::deserialize_from_json(json).unwrap();
    assert!(nodes.is_empty());
  }

  #[test]
  fn test_deserialize_from_json_single_root_node() {
    let json = r#"[{"type": "root", "id": "root-1", "value": null}]"#.to_string();
    let nodes = BundleGraphFromJs::deserialize_from_json(json).unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].id(), "root-1");
    assert!(matches!(nodes[0], BundleGraphNode::Root(_)));
  }

  #[test]
  fn test_deserialize_from_json_multiple_nodes() {
    let json = r#"[
      {"type": "root", "id": "root", "value": null},
      {"type": "entry_specifier", "id": "es-1", "value": "/src/index.js"}
    ]"#
      .to_string();
    let nodes = BundleGraphFromJs::deserialize_from_json(json).unwrap();
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0].id(), "root");
    assert_eq!(nodes[1].id(), "es-1");
  }

  #[test]
  fn test_deserialize_from_json_invalid_json() {
    let json = "not valid json".to_string();
    let result = BundleGraphFromJs::deserialize_from_json(json);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Failed to parse bundle graph JSON"));
  }

  #[test]
  fn test_deserialize_from_json_invalid_node_type() {
    let json = r#"[{"type": "invalid_type", "id": "test"}]"#.to_string();
    let result = BundleGraphFromJs::deserialize_from_json(json);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Failed to deserialize node"));
  }

  #[test]
  fn test_deserialize_from_json_missing_required_field() {
    // Missing "id" field
    let json = r#"[{"type": "root", "value": null}]"#.to_string();
    let result = BundleGraphFromJs::deserialize_from_json(json);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Failed to deserialize node"));
  }

  #[test]
  fn test_deserialize_from_json_integration_with_graph() {
    // Test that deserialized nodes can be used to create a graph
    let json = r#"[
      {"type": "root", "id": "root", "value": null},
      {"type": "entry_specifier", "id": "es-1", "value": "/src/index.js"}
    ]"#
      .to_string();
    let nodes = BundleGraphFromJs::deserialize_from_json(json).unwrap();
    let edges = vec![(0, 1, 1u8)];

    let graph = BundleGraphFromJs::new(nodes, edges);
    // Graph should be created successfully (no bundles in this case)
    assert!(graph.get_bundles().is_empty());
  }
}
