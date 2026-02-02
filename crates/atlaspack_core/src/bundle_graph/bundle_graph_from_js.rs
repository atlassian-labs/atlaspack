use anyhow::anyhow;
use std::collections::HashMap;

use petgraph::{
  Direction,
  graph::NodeIndex,
  prelude::StableDiGraph,
  visit::{Dfs, EdgeRef},
};
use rayon::prelude::*;

use crate::{
  bundle_graph::bundle_graph::BundleGraph,
  types::{Asset, Bundle, BundleGraphEdgeType, BundleGraphNode},
};

pub struct BundleGraphFromJs {
  graph: StableDiGraph<BundleGraphNode, BundleGraphEdgeType>,
  /// Content key (e.g. bundle.id, asset.id) -> NodeIndex. Mirrors JS ContentGraph._contentKeyToNodeId.
  node_by_key: HashMap<String, NodeIndex>,
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
    let node_by_key = graph
      .node_indices()
      .map(|idx| {
        let id = graph.node_weight(idx).unwrap().id().to_string();
        (id, idx)
      })
      .collect();
    BundleGraphFromJs { graph, node_by_key }
  }

  /// Returns the node index for the given content key (e.g. bundle.id, asset.id).
  /// Equivalent to JS ContentGraph.getNodeIdByContentKey(contentKey).
  pub fn get_node_id(&self, key: &str) -> Option<&NodeIndex> {
    self.node_by_key.get(key)
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
  fn get_bundle_assets(&self, bundle: &Bundle) -> anyhow::Result<Vec<&Asset>> {
    let bundle_node_id = self
      .get_node_id(&bundle.id)
      .ok_or(anyhow!("Bundle {} not found in bundle graph", bundle.id))?;

    let bundle_assets: Vec<&Asset> = self
      .graph
      .edges_directed(*bundle_node_id, Direction::Outgoing)
      .filter_map(|e| match (e.weight(), self.graph.node_weight(e.target())) {
        (BundleGraphEdgeType::Contains, Some(BundleGraphNode::Asset(an))) => Some(&an.value),
        _ => None,
      })
      .collect::<Vec<&Asset>>();
    Ok(bundle_assets)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::types::{Asset, Dependency};
  use crate::types::{
    AssetNode, BundleGraphEdgeType, BundleNode, DependencyNode, Environment, FileType, Priority,
    RootNode, SpecifierType, Target,
  };
  use pretty_assertions::assert_eq;
  use std::path::PathBuf;
  use std::sync::Arc;

  fn create_test_bundle(
    id: &str,
    name: Option<&str>,
    entry_asset_ids: Option<Vec<String>>,
  ) -> Bundle {
    Bundle {
      id: id.to_string(),
      name: name.map(|s| s.to_string()),
      bundle_behavior: None,
      bundle_type: FileType::Js,
      entry_asset_ids: entry_asset_ids.unwrap_or_default(),
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

  fn create_test_bundle_node(
    id: &str,
    name: Option<&str>,
    entry_asset_ids: Option<Vec<String>>,
  ) -> BundleNode {
    BundleNode {
      id: id.to_string(),
      value: create_test_bundle(id, name, entry_asset_ids),
    }
  }

  fn create_test_root_node() -> RootNode {
    RootNode {
      id: "root".to_string(),
      value: None,
    }
  }

  /// Returns asset ids from get_bundle_assets result for assertions.
  fn asset_ids<'a>(assets: &'a [&'a Asset]) -> Vec<&'a str> {
    assets.iter().map(|a| a.id.as_str()).collect()
  }

  /// Asserts that the result contains exactly the expected asset ids (set equality).
  fn assert_contains_asset_ids(result: &[&Asset], expected: &[&str]) {
    let mut result_ids = asset_ids(result);
    result_ids.sort();
    let mut expected_sorted: Vec<&str> = expected.to_vec();
    expected_sorted.sort();
    assert_eq!(result_ids, expected_sorted, "asset set mismatch");
  }

  /// Asserts that entry assets appear in result in the same order as bundle.entry_asset_ids.
  fn assert_entry_asset_order(bundle: &Bundle, result: &[&Asset]) {
    let result_ids = asset_ids(result);
    let mut last_pos = 0usize;
    for entry_id in &bundle.entry_asset_ids {
      if let Some(p) = result_ids.iter().position(|id| *id == entry_id.as_str()) {
        assert!(
          p >= last_pos,
          "entry_asset_ids order violated: {} should appear at or after position {}",
          entry_id,
          last_pos
        );
        last_pos = p;
      }
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
      BundleGraphNode::Bundle(create_test_bundle_node("bundle1", Some("main.js"), None)),
    ];
    let edges = vec![(0, 1, BundleGraphEdgeType::Null)]; // Edge from root to bundle

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
      BundleGraphNode::Bundle(create_test_bundle_node("bundle1", Some("main.js"), None)),
      BundleGraphNode::Bundle(create_test_bundle_node("bundle2", Some("chunk-a.js"), None)),
      BundleGraphNode::Bundle(create_test_bundle_node("bundle3", Some("chunk-b.js"), None)),
    ];
    let edges = vec![
      (0, 1, BundleGraphEdgeType::Null),     // root -> bundle1
      (1, 2, BundleGraphEdgeType::Contains), // bundle1 -> bundle2
      (1, 3, BundleGraphEdgeType::Contains), // bundle1 -> bundle3
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
      BundleGraphNode::Bundle(create_test_bundle_node("bundle1", Some("main.js"), None)),
      BundleGraphNode::Dependency(create_test_dependency_node("dep1")),
      BundleGraphNode::Bundle(create_test_bundle_node("bundle2", Some("chunk.js"), None)),
    ];
    let edges = vec![
      (0, 1, BundleGraphEdgeType::Null),     // root -> asset
      (0, 2, BundleGraphEdgeType::Null),     // root -> bundle1
      (2, 3, BundleGraphEdgeType::Contains), // bundle1 -> dependency
      (3, 4, BundleGraphEdgeType::Contains), // dependency -> bundle2
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
      BundleGraphNode::Bundle(create_test_bundle_node("bundle1", Some("main.js"), None)),
      BundleGraphNode::Bundle(create_test_bundle_node("bundle2", Some("orphan.js"), None)),
    ];
    // Only edge from root to bundle1, bundle2 is disconnected
    let edges = vec![(0, 1, BundleGraphEdgeType::Null)];

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

    let bundle = BundleGraphNode::Bundle(create_test_bundle_node("b1", Some("test.js"), None));
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
    let edges = vec![(0, 1, BundleGraphEdgeType::Null)];

    let graph = BundleGraphFromJs::new(nodes, edges);
    // Graph should be created successfully (no bundles in this case)
    assert!(graph.get_bundles().is_empty());
  }

  #[test]
  fn test_get_bundle_assets_handles_empty_bundle_gracefully() {
    let nodes = vec![BundleGraphNode::Bundle(create_test_bundle_node(
      "empty_bundle",
      None,
      Some(vec![]),
    ))];
    let edges: Vec<(u32, u32, BundleGraphEdgeType)> = vec![];
    let graph = BundleGraphFromJs::new(nodes, edges);
    let bundles = graph.get_bundles();
    let bundle = bundles[0];
    let bundle_assets = graph.get_bundle_assets(bundle).unwrap();

    assert!(bundle_assets.is_empty());
  }

  #[test]
  fn test_get_bundle_assets_skips_nodes_not_contained_in_bundle() {
    let nodes = vec![
      BundleGraphNode::Bundle(create_test_bundle_node(
        "bundle1",
        None,
        Some(vec!["asset1".to_string()]),
      )),
      BundleGraphNode::Asset(create_test_asset_node("asset1")),
      BundleGraphNode::Asset(create_test_asset_node("asset2")),
      BundleGraphNode::Asset(create_test_asset_node("asset3")),
      BundleGraphNode::Asset(create_test_asset_node("external")),
      BundleGraphNode::Dependency(create_test_dependency_node("dep1")),
      BundleGraphNode::Dependency(create_test_dependency_node("dep2")),
      BundleGraphNode::Dependency(create_test_dependency_node("external_dep")),
    ];
    let edges = vec![
      (0, 1, BundleGraphEdgeType::Contains),
      (0, 1, BundleGraphEdgeType::Null),
      (0, 2, BundleGraphEdgeType::Contains),
      (0, 3, BundleGraphEdgeType::Contains),
      (0, 5, BundleGraphEdgeType::Contains),
      (0, 6, BundleGraphEdgeType::Contains),
      (1, 5, BundleGraphEdgeType::Null),
      (5, 2, BundleGraphEdgeType::Null),
      (2, 6, BundleGraphEdgeType::Null),
      (6, 3, BundleGraphEdgeType::Null),
      (1, 7, BundleGraphEdgeType::Null),
      (7, 4, BundleGraphEdgeType::Null),
    ];
    let graph = BundleGraphFromJs::new(nodes, edges);
    let bundles = graph.get_bundles();
    let bundle = bundles[0];
    let bundle_assets = graph.get_bundle_assets(bundle).unwrap();

    assert_contains_asset_ids(&bundle_assets, &["asset1", "asset2", "asset3"]);
    assert!(!asset_ids(&bundle_assets).contains(&"external"));
  }

  #[test]
  fn test_get_bundle_assets_handles_bundle_with_single_asset() {
    let nodes = vec![
      BundleGraphNode::Bundle(create_test_bundle_node(
        "bundle1",
        None,
        Some(vec!["asset3".to_string()]),
      )),
      BundleGraphNode::Asset(create_test_asset_node("asset3")),
    ];
    let edges = vec![
      (0, 1, BundleGraphEdgeType::Contains),
      (0, 1, BundleGraphEdgeType::Null),
    ];
    let graph = BundleGraphFromJs::new(nodes, edges);
    let bundles = graph.get_bundles();
    let bundle = bundles[0];
    let bundle_assets = graph.get_bundle_assets(bundle).unwrap();

    assert_contains_asset_ids(&bundle_assets, &["asset3"]);
    assert_entry_asset_order(bundle, &bundle_assets);
  }

  #[test]
  fn test_get_bundle_assets_returns_all_assets_in_bundle() {
    let nodes = vec![
      BundleGraphNode::Bundle(create_test_bundle_node(
        "bundle1",
        None,
        Some(vec!["asset1".to_string()]),
      )),
      BundleGraphNode::Asset(create_test_asset_node("asset1")),
      BundleGraphNode::Asset(create_test_asset_node("asset2")),
      BundleGraphNode::Dependency(create_test_dependency_node("dep1")),
      BundleGraphNode::Dependency(create_test_dependency_node("dep2")),
    ];
    let edges = vec![
      (0, 1, BundleGraphEdgeType::Contains),
      (0, 1, BundleGraphEdgeType::Null),
      (0, 2, BundleGraphEdgeType::Contains),
      (0, 3, BundleGraphEdgeType::Contains),
      (0, 4, BundleGraphEdgeType::Contains),
      (1, 3, BundleGraphEdgeType::Null),
      (3, 2, BundleGraphEdgeType::Null),
      (2, 4, BundleGraphEdgeType::Null),
    ];
    let graph = BundleGraphFromJs::new(nodes, edges);
    let bundles = graph.get_bundles();
    let bundle = bundles[0];
    let bundle_assets = graph.get_bundle_assets(bundle).unwrap();

    assert_contains_asset_ids(&bundle_assets, &["asset1", "asset2"]);
    assert_entry_asset_order(bundle, &bundle_assets);
  }
}
