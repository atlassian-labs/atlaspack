use anyhow::anyhow;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use petgraph::{
  Direction,
  graph::NodeIndex,
  prelude::StableDiGraph,
  visit::{Control, Dfs, DfsEvent, EdgeRef, depth_first_search},
};
use rayon::prelude::*;

use crate::{
  bundle_graph::bundle_graph::BundleGraph,
  types::{self, Asset, Bundle, BundleGraphEdgeType, BundleGraphNode, Dependency, Environment},
};

type BundleGraphNodeId = String;

#[derive(Default)]
pub struct BundleGraphFromJs {
  graph: StableDiGraph<BundleGraphNode, BundleGraphEdgeType>,
  /// Content key (e.g. bundle.id, asset.id) -> NodeIndex. Mirrors JS ContentGraph._contentKeyToNodeId.
  nodes_by_key: HashMap<BundleGraphNodeId, NodeIndex>,
  /// Maps full asset IDs (16-character hex strings) to shortened public IDs (base62-encoded).
  public_id_by_asset_id: HashMap<String, String>,
  /// Maps environment IDs to Environment objects for lookup
  _environments_by_id: HashMap<String, Arc<Environment>>,
  /// Cache of (bundle_node, asset_node) pairs with Contains edges for fast lookup
  bundle_contains_cache: HashMap<NodeIndex, HashSet<NodeIndex>>,
}

impl BundleGraphFromJs {
  pub fn new(
    nodes: Vec<BundleGraphNode>,
    edges: Vec<(u32, u32, BundleGraphEdgeType)>,
    public_id_by_asset_id: HashMap<String, String>,
    environments: Vec<Environment>,
  ) -> Self {
    // Build environment lookup map
    let environments_by_id: HashMap<String, Arc<Environment>> = environments
      .into_iter()
      .map(|env| {
        let id = env.id();
        (id, Arc::new(env))
      })
      .collect();

    let mut graph = StableDiGraph::new();
    let mut nodes_by_key = HashMap::new();
    for node in nodes {
      let id = node.id().to_string();
      let idx = graph.add_node(node);
      nodes_by_key.insert(id, idx);
    }

    // Build bundle_contains_cache while adding edges
    let mut bundle_contains_cache: HashMap<NodeIndex, HashSet<NodeIndex>> = HashMap::new();

    for edge in edges {
      let from_idx = NodeIndex::new(edge.0 as usize);
      let to_idx = NodeIndex::new(edge.1 as usize);
      let edge_type = edge.2;

      graph.add_edge(from_idx, to_idx, edge_type);

      // Cache Contains edges for fast lookup
      if edge_type == BundleGraphEdgeType::Contains {
        bundle_contains_cache
          .entry(from_idx)
          .or_default()
          .insert(to_idx);
      }
    }

    BundleGraphFromJs {
      graph,
      nodes_by_key,
      public_id_by_asset_id,
      _environments_by_id: environments_by_id,
      bundle_contains_cache,
    }
  }

  /// Returns all bundles reachable from the root node via DFS traversal.
  pub fn get_bundles(&self) -> Vec<&Bundle> {
    let Some(root_idx) = self.nodes_by_key.get("root") else {
      tracing::debug!("get_bundles: No root node found");
      return Vec::new();
    };

    let mut bundles = Vec::new();
    let mut dfs = Dfs::new(&self.graph, *root_idx);
    while let Some(node_idx) = dfs.next(&self.graph) {
      if let Some(BundleGraphNode::Bundle(bundle_node)) = self.graph.node_weight(node_idx) {
        bundles.push(&bundle_node.value);
      }
    }
    bundles
  }

  /// Returns the node index for the given content key (e.g. bundle.id, asset.id).
  /// Equivalent to JS ContentGraph.getNodeIdByContentKey(contentKey).
  pub fn get_node_id(&self, key: &str) -> Option<&NodeIndex> {
    self.nodes_by_key.get(key)
  }

  #[tracing::instrument(
    level = "info",
    skip_all,
    fields(json_size = nodes_json.len())
  )]
  pub fn deserialize_from_json(
    nodes_json: String,
    environments: &[Environment],
  ) -> anyhow::Result<Vec<BundleGraphNode>> {
    // Build environment lookup map
    let environments_by_id: HashMap<String, Arc<Environment>> = environments
      .iter()
      .map(|env| {
        let id = env.id();
        (id, Arc::new(env.clone()))
      })
      .collect();

    // Parse JSON to Vec<Value> first (fast), then parallelize node deserialization
    let json_values: Vec<serde_json::Value> = serde_json::from_str(&nodes_json)
      .map_err(|e| anyhow!("Failed to parse bundle graph JSON: {}", e))?;

    // Parallelize the deserialization of individual nodes using rayon
    let nodes: Vec<BundleGraphNode> = json_values
      .into_par_iter()
      .map(|value| {
        Self::deserialize_node_with_env_lookup(value, &environments_by_id)
          .map_err(|e| anyhow!("Failed to deserialize node: {}", e))
      })
      .collect::<anyhow::Result<Vec<_>>>()?;

    // Count node types for debugging
    let mut counts = std::collections::HashMap::new();
    for node in &nodes {
      let type_name = match node {
        BundleGraphNode::Root(_) => "Root",
        BundleGraphNode::Asset(_) => "Asset",
        BundleGraphNode::Dependency(_) => "Dependency",
        BundleGraphNode::Bundle(_) => "Bundle",
        BundleGraphNode::BundleGroup(_) => "BundleGroup",
        BundleGraphNode::EntryFile(_) => "EntryFile",
        BundleGraphNode::EntrySpecifier(_) => "EntrySpecifier",
      };
      *counts.entry(type_name).or_insert(0) += 1;
    }

    tracing::debug!(
      "Node type counts: Root={}, Asset={}, Dependency={}, Bundle={}, BundleGroup={}, EntryFile={}, EntrySpecifier={}",
      counts.get("Root").unwrap_or(&0),
      counts.get("Asset").unwrap_or(&0),
      counts.get("Dependency").unwrap_or(&0),
      counts.get("Bundle").unwrap_or(&0),
      counts.get("BundleGroup").unwrap_or(&0),
      counts.get("EntryFile").unwrap_or(&0),
      counts.get("EntrySpecifier").unwrap_or(&0),
    );

    tracing::Span::current().record("nodes", nodes.len());
    Ok(nodes)
  }

  /// Deserialize a single node, replacing environment ID strings with Arc<Environment> references
  fn deserialize_node_with_env_lookup(
    mut value: serde_json::Value,
    environments_by_id: &HashMap<String, Arc<Environment>>,
  ) -> anyhow::Result<BundleGraphNode> {
    // Check if this node has an env field that needs to be resolved
    if let Some(node_value) = value.get_mut("value")
      && let Some(env_id) = node_value.get("env").and_then(|v| v.as_str())
    {
      // Look up the environment and replace the ID with the full object
      let env = environments_by_id
        .get(env_id)
        .ok_or_else(|| anyhow!("Environment ID not found: {}", env_id))?;

      // Serialize the environment and replace the env field
      let env_value = serde_json::to_value(&**env)
        .map_err(|e| anyhow!("Failed to serialize environment: {}", e))?;
      node_value["env"] = env_value;
    }

    serde_json::from_value::<BundleGraphNode>(value)
      .map_err(|e| anyhow!("Failed to deserialize node: {}", e))
  }
}

impl BundleGraph for BundleGraphFromJs {
  fn get_bundle_by_id(&self, id: &str) -> Option<&Bundle> {
    if let Some(node_idx) = self.nodes_by_key.get(id)
      && let Some(node) = self.graph.node_weight(*node_idx)
    {
      return match node {
        BundleGraphNode::Bundle(node) => Some(&node.value),
        _ => None,
      };
    }
    None
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

  fn get_public_asset_id(&self, asset_id: &str) -> Option<&str> {
    self.public_id_by_asset_id.get(asset_id).map(|s| s.as_str())
  }

  #[tracing::instrument(level = "debug", skip_all)]
  fn get_dependencies(&self, asset: &Asset) -> anyhow::Result<Vec<&types::Dependency>> {
    let asset_node = self.nodes_by_key.get(&asset.id).unwrap();

    self
      .graph
      .edges_directed(*asset_node, Direction::Outgoing)
      .filter(|edge| *edge.weight() == BundleGraphEdgeType::Null)
      .map(|edge| {
        let node = self.graph.node_weight(edge.target()).unwrap();
        let BundleGraphNode::Dependency(dependency) = node else {
          return Err(anyhow!("Expected dependency node, got {:?}", node));
        };
        Ok(&dependency.value)
      })
      .collect()
  }

  #[tracing::instrument(level = "debug", skip_all)]
  fn get_resolved_asset(
    &self,
    dependency: &Dependency,
    bundle: &Bundle,
  ) -> anyhow::Result<Option<&Asset>> {
    let dependency_node = self.nodes_by_key.get(&dependency.id).ok_or(anyhow!(
      "Dependency {} not found in bundle graph",
      dependency.id
    ))?;

    let bundle_node = self
      .nodes_by_key
      .get(&bundle.id)
      .ok_or(anyhow!("Bundle {} not found in bundle graph", bundle.id))?;

    let bundle_type = &bundle.bundle_type;

    // Single pass: prioritize by: contains edge > type match > first asset
    let mut first_asset: Option<&Asset> = None;
    let mut type_matched_asset: Option<&Asset> = None;

    for asset_node in self
      .graph
      .neighbors_directed(*dependency_node, Direction::Outgoing)
    {
      // Only process asset nodes
      let Some(BundleGraphNode::Asset(asset)) = self.graph.node_weight(asset_node) else {
        continue;
      };

      // Highest priority: contains edge
      // Use cached lookup instead of edges_connecting (O(1) vs O(E'))
      if self
        .bundle_contains_cache
        .get(bundle_node)
        .map(|assets| assets.contains(&asset_node))
        .unwrap_or(false)
      {
        return Ok(Some(&asset.value));
      }

      // Medium priority: type match (save for later)
      if type_matched_asset.is_none() && asset.value.file_type == *bundle_type {
        type_matched_asset = Some(&asset.value);
      }

      // Lowest priority: first asset (save for later)
      if first_asset.is_none() {
        first_asset = Some(&asset.value);
      }
    }

    // If found via direct neighbors, return in priority order
    if let Some(asset) = type_matched_asset.or(first_asset) {
      return Ok(Some(asset));
    }

    // Fallback: traverse via References edges to find any reachable assets
    // This matches the TypeScript implementation that traverses with skipChildren control
    let mut type_matched_fallback: Option<&Asset> = None;
    let mut first_fallback: Option<&Asset> = None;

    // Use depth_first_search with Control to properly skip children
    depth_first_search(&self.graph, Some(*dependency_node), |event| {
      match event {
        DfsEvent::Discover(node_idx, _) => {
          // Skip the dependency node itself
          if node_idx == *dependency_node {
            return Control::Continue;
          }

          match self.graph.node_weight(node_idx) {
            Some(BundleGraphNode::Asset(asset)) => {
              // Found an asset - check if it matches bundle type
              if type_matched_fallback.is_none() && asset.value.file_type == *bundle_type {
                type_matched_fallback = Some(&asset.value);
              }
              if first_fallback.is_none() {
                first_fallback = Some(&asset.value);
              }

              // Early exit if we found a type match
              if type_matched_fallback.is_some() {
                return Control::Break(());
              }
              Control::Continue
            }
            Some(BundleGraphNode::Dependency(_)) => {
              // Continue traversal through dependency nodes
              Control::Continue
            }
            _ => {
              // Skip children for non-asset, non-dependency nodes
              // This matches the TypeScript behavior: traversal.skipChildren()
              Control::Prune
            }
          }
        }
        _ => Control::Continue,
      }
    });

    // Return type-matched asset or first asset found
    Ok(type_matched_fallback.or(first_fallback))
  }

  fn is_dependency_skipped(&self, dependency: &Dependency) -> bool {
    let dependency_node = self.nodes_by_key.get(&dependency.id).unwrap();
    let node = self.graph.node_weight(*dependency_node).unwrap();
    let BundleGraphNode::Dependency(dependency) = node else {
      return false;
    };
    dependency.deferred || dependency.excluded
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
    let graph = BundleGraphFromJs::new(vec![], vec![], HashMap::new(), vec![]);
    assert!(graph.get_bundles().is_empty());
  }

  #[test]
  fn test_bundle_graph_from_js_single_bundle() {
    let nodes = vec![
      BundleGraphNode::Root(create_test_root_node()),
      BundleGraphNode::Bundle(create_test_bundle_node("bundle1", "main.js")),
    ];
    let edges = vec![(0, 1, BundleGraphEdgeType::Null)]; // Edge from root to bundle

    let graph = BundleGraphFromJs::new(nodes, edges, HashMap::new(), vec![Environment::default()]);
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
      (0, 1, BundleGraphEdgeType::Null),     // root -> bundle1
      (1, 2, BundleGraphEdgeType::Contains), // bundle1 -> bundle2
      (1, 3, BundleGraphEdgeType::Contains), // bundle1 -> bundle3
    ];

    let graph = BundleGraphFromJs::new(nodes, edges, HashMap::new(), vec![Environment::default()]);
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
      (0, 1, BundleGraphEdgeType::Null),     // root -> asset
      (0, 2, BundleGraphEdgeType::Null),     // root -> bundle1
      (2, 3, BundleGraphEdgeType::Contains), // bundle1 -> dependency
      (3, 4, BundleGraphEdgeType::Contains), // dependency -> bundle2
    ];

    let graph = BundleGraphFromJs::new(nodes, edges, HashMap::new(), vec![Environment::default()]);
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
    let edges = vec![(0, 1, BundleGraphEdgeType::Null)];

    let graph = BundleGraphFromJs::new(nodes, edges, HashMap::new(), vec![Environment::default()]);
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
    let nodes = BundleGraphFromJs::deserialize_from_json(json, &[]).unwrap();
    assert!(nodes.is_empty());
  }

  #[test]
  fn test_deserialize_from_json_single_root_node() {
    let json = r#"[{"type": "root", "id": "root-1", "value": null}]"#.to_string();
    let nodes = BundleGraphFromJs::deserialize_from_json(json, &[]).unwrap();
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
    let nodes = BundleGraphFromJs::deserialize_from_json(json, &[]).unwrap();
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0].id(), "root");
    assert_eq!(nodes[1].id(), "es-1");
  }

  #[test]
  fn test_deserialize_from_json_invalid_json() {
    let json = "not valid json".to_string();
    let result = BundleGraphFromJs::deserialize_from_json(json, &[]);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Failed to parse bundle graph JSON"));
  }

  #[test]
  fn test_deserialize_from_json_invalid_node_type() {
    let json = r#"[{"type": "invalid_type", "id": "test"}]"#.to_string();
    let result = BundleGraphFromJs::deserialize_from_json(json, &[]);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Failed to deserialize node"));
  }

  #[test]
  fn test_deserialize_from_json_missing_required_field() {
    // Missing "id" field
    let json = r#"[{"type": "root", "value": null}]"#.to_string();
    let result = BundleGraphFromJs::deserialize_from_json(json, &[]);
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
    let nodes = BundleGraphFromJs::deserialize_from_json(json, &[]).unwrap();
    let edges = vec![(0, 1, BundleGraphEdgeType::Null)];

    let graph = BundleGraphFromJs::new(nodes, edges, HashMap::new(), vec![]);
    // Graph should be created successfully (no bundles in this case)
    assert!(graph.get_bundles().is_empty());
  }

  #[test]
  fn test_get_public_asset_id() {
    let mut public_id_by_asset_id = HashMap::new();
    public_id_by_asset_id.insert("abc123def456".to_string(), "8LVYC".to_string());
    public_id_by_asset_id.insert("xyz789uvw012".to_string(), "d7Pd5".to_string());

    let graph = BundleGraphFromJs::new(vec![], vec![], public_id_by_asset_id, vec![]);

    assert_eq!(graph.get_public_asset_id("abc123def456"), Some("8LVYC"));
    assert_eq!(graph.get_public_asset_id("xyz789uvw012"), Some("d7Pd5"));
    assert_eq!(graph.get_public_asset_id("nonexistent"), None);
  }

  #[test]
  fn test_get_dependencies_returns_outgoing_null_edges() {
    let asset1 = create_test_asset_node("asset1");
    let dep1 = create_test_dependency_node("dep1");
    let dep2 = create_test_dependency_node("dep2");

    let nodes = vec![
      BundleGraphNode::Asset(asset1.clone()),
      BundleGraphNode::Dependency(dep1.clone()),
      BundleGraphNode::Dependency(dep2.clone()),
    ];

    // asset1 -> dep1 (Null edge)
    // asset1 -> dep2 (Null edge)
    let edges = vec![
      (0, 1, BundleGraphEdgeType::Null),
      (0, 2, BundleGraphEdgeType::Null),
    ];

    let graph = BundleGraphFromJs::new(nodes, edges, HashMap::new(), vec![]);

    let deps = graph.get_dependencies(&asset1.value).unwrap();
    assert_eq!(deps.len(), 2);
    let dep_ids: Vec<&str> = deps.iter().map(|d| d.id.as_str()).collect();
    assert!(dep_ids.contains(&"dep1"));
    assert!(dep_ids.contains(&"dep2"));
  }

  #[test]
  fn test_get_dependencies_filters_non_null_edges() {
    let asset1 = create_test_asset_node("asset1");
    let dep1 = create_test_dependency_node("dep1");
    let dep2 = create_test_dependency_node("dep2");

    let nodes = vec![
      BundleGraphNode::Asset(asset1.clone()),
      BundleGraphNode::Dependency(dep1.clone()),
      BundleGraphNode::Dependency(dep2.clone()),
    ];

    // asset1 -> dep1 (Null edge - should be included)
    // asset1 -> dep2 (References edge - should be filtered out)
    let edges = vec![
      (0, 1, BundleGraphEdgeType::Null),
      (0, 2, BundleGraphEdgeType::References),
    ];

    let graph = BundleGraphFromJs::new(nodes, edges, HashMap::new(), vec![]);

    let deps = graph.get_dependencies(&asset1.value).unwrap();
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0].id, "dep1");
  }

  #[test]
  fn test_get_dependencies_empty_when_no_edges() {
    let asset1 = create_test_asset_node("asset1");

    let nodes = vec![BundleGraphNode::Asset(asset1.clone())];
    let edges = vec![];

    let graph = BundleGraphFromJs::new(nodes, edges, HashMap::new(), vec![]);

    let deps = graph.get_dependencies(&asset1.value).unwrap();
    assert!(deps.is_empty());
  }

  #[test]
  fn test_is_dependency_skipped_when_deferred() {
    let mut dep_node = create_test_dependency_node("dep1");
    dep_node.deferred = true;

    let nodes = vec![BundleGraphNode::Dependency(dep_node.clone())];

    let graph = BundleGraphFromJs::new(nodes, vec![], HashMap::new(), vec![]);

    assert!(graph.is_dependency_skipped(&dep_node.value));
  }

  #[test]
  fn test_is_dependency_skipped_when_excluded() {
    let mut dep_node = create_test_dependency_node("dep1");
    dep_node.excluded = true;

    let nodes = vec![BundleGraphNode::Dependency(dep_node.clone())];

    let graph = BundleGraphFromJs::new(nodes, vec![], HashMap::new(), vec![]);

    assert!(graph.is_dependency_skipped(&dep_node.value));
  }

  #[test]
  fn test_is_dependency_skipped_when_not_skipped() {
    let dep_node = create_test_dependency_node("dep1");

    let nodes = vec![BundleGraphNode::Dependency(dep_node.clone())];

    let graph = BundleGraphFromJs::new(nodes, vec![], HashMap::new(), vec![]);

    assert!(!graph.is_dependency_skipped(&dep_node.value));
  }

  #[test]
  fn test_get_resolved_asset_with_contains_edge() {
    let bundle = create_test_bundle_node("bundle1", "main.js");
    let dep = create_test_dependency_node("dep1");
    let asset = create_test_asset_node("asset1");

    let nodes = vec![
      BundleGraphNode::Bundle(bundle),
      BundleGraphNode::Dependency(dep),
      BundleGraphNode::Asset(asset),
    ];

    // dep -> asset (outgoing from dep)
    // bundle -> asset (Contains edge - highest priority)
    let edges = vec![
      (1, 2, BundleGraphEdgeType::Null),
      (0, 2, BundleGraphEdgeType::Contains),
    ];

    let graph = BundleGraphFromJs::new(nodes, edges, HashMap::new(), vec![]);

    // Get references to the nodes for testing
    let bundle_ref = graph.get_bundle_by_id("bundle1").unwrap();
    let dep_value = &create_test_dependency_node("dep1").value;

    let resolved = graph.get_resolved_asset(dep_value, bundle_ref).unwrap();
    assert!(resolved.is_some());
    assert_eq!(resolved.unwrap().id, "asset1");
  }

  #[test]
  fn test_get_resolved_asset_with_type_match() {
    let mut bundle = create_test_bundle("bundle1", "main.js");
    bundle.bundle_type = FileType::Js;

    let dep = create_test_dependency_node("dep1");
    let mut asset1 = create_test_asset_node("asset1");
    asset1.value.file_type = FileType::Css;
    let mut asset2 = create_test_asset_node("asset2");
    asset2.value.file_type = FileType::Js; // Matches bundle type

    let nodes = vec![
      BundleGraphNode::Bundle(BundleNode {
        id: bundle.id.clone(),
        value: bundle.clone(),
      }),
      BundleGraphNode::Dependency(dep),
      BundleGraphNode::Asset(asset1),
      BundleGraphNode::Asset(asset2),
    ];

    // dep -> asset1 (CSS, doesn't match)
    // dep -> asset2 (JS, matches bundle type)
    let edges = vec![
      (1, 2, BundleGraphEdgeType::Null),
      (1, 3, BundleGraphEdgeType::Null),
    ];

    let graph = BundleGraphFromJs::new(nodes, edges, HashMap::new(), vec![]);

    // Get reference for testing
    let dep_value = &create_test_dependency_node("dep1").value;
    let resolved = graph.get_resolved_asset(dep_value, &bundle).unwrap();
    assert!(resolved.is_some());
    // Should prefer asset2 due to type match
    assert_eq!(resolved.unwrap().id, "asset2");
  }

  #[test]
  fn test_get_resolved_asset_returns_none_when_not_found() {
    let bundle = create_test_bundle("bundle1", "main.js");
    let dep = create_test_dependency_node("dep1");

    let nodes = vec![
      BundleGraphNode::Bundle(BundleNode {
        id: bundle.id.clone(),
        value: bundle.clone(),
      }),
      BundleGraphNode::Dependency(dep),
    ];

    let graph = BundleGraphFromJs::new(nodes, vec![], HashMap::new(), vec![]);

    let dep_value = &create_test_dependency_node("dep1").value;
    let resolved = graph.get_resolved_asset(dep_value, &bundle).unwrap();
    assert!(resolved.is_none());
  }

  #[test]
  fn test_get_resolved_asset_with_references_edge_traversal() {
    let bundle = create_test_bundle("bundle1", "main.js");
    let dep = create_test_dependency_node("dep1");
    let intermediate_dep = create_test_dependency_node("dep2");
    let asset = create_test_asset_node("asset1");

    let nodes = vec![
      BundleGraphNode::Bundle(BundleNode {
        id: bundle.id.clone(),
        value: bundle.clone(),
      }),
      BundleGraphNode::Dependency(dep),
      BundleGraphNode::Dependency(intermediate_dep),
      BundleGraphNode::Asset(asset),
    ];

    // dep1 -> dep2 (intermediate dependency)
    // dep2 -> asset1
    let edges = vec![
      (1, 2, BundleGraphEdgeType::References),
      (2, 3, BundleGraphEdgeType::Null),
    ];

    let graph = BundleGraphFromJs::new(nodes, edges, HashMap::new(), vec![]);

    let dep_value = &create_test_dependency_node("dep1").value;
    let resolved = graph.get_resolved_asset(dep_value, &bundle).unwrap();
    // Should find asset via traversal
    assert!(resolved.is_some());
    assert_eq!(resolved.unwrap().id, "asset1");
  }
}
