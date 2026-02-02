use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use async_trait::async_trait;
use serde::Serialize;

use atlaspack_core::asset_graph::{AssetGraph, AssetGraphNode};
use atlaspack_core::bundle_graph::native_bundle_graph::generate_public_id;
use atlaspack_core::bundle_graph::NativeBundleGraph;
use atlaspack_core::hash::hash_string;
use atlaspack_core::types::{Bundle, FileType, Target};

use crate::request_tracker::{Request, ResultAndInvalidations, RunRequestContext, RunRequestError};

use super::RequestResult;

/// The BundleGraphRequest is responsible for building the BundleGraph from the AssetGraph.
///
/// For milestone 1, this implements a simplified "monolithic" bundling algorithm where
/// all reachable JS assets are assigned to a single bundle.
#[derive(Debug, Default)]
pub struct BundleGraphRequest {
  /// The asset graph to bundle
  pub asset_graph: Arc<AssetGraph>,
}

impl Hash for BundleGraphRequest {
  fn hash<H: Hasher>(&self, _state: &mut H) {
    // Hash returns nothing here as every BundleGraphRequest should have the same
    // ID in the end (similar to AssetGraphRequest).
  }
}

/// Output of the BundleGraphRequest.
///
/// This is shaped similarly to the JS-side bundle graph representation, but optimized for
/// incremental bootstrapping (like `AssetGraphRequestRust`).
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleGraphRequestOutput {
  /// Bundle-specific nodes (bundle + bundle_group) as JSON strings.
  pub bundle_nodes: Vec<String>,

  /// Bundle-specific edges as flat numeric triples: [from, to, type, ...].
  pub bundle_edges: Vec<u32>,

  /// Maps full asset IDs to concise public IDs.
  pub public_id_by_asset_id: HashMap<String, String>,

  /// Set of all assigned asset public IDs.
  pub asset_public_ids: Vec<String>,

  /// Whether Rust had a previous bundle graph that could be incrementally updated.
  pub had_previous_graph: bool,
}

impl BundleGraphRequestOutput {
  pub fn from_bundle_graph(bundle_graph: &NativeBundleGraph) -> Self {
    Self {
      bundle_nodes: vec![],
      bundle_edges: vec![],
      public_id_by_asset_id: bundle_graph.public_id_by_asset_id.clone(),
      asset_public_ids: bundle_graph.asset_public_ids.iter().cloned().collect(),
      had_previous_graph: false,
    }
  }
}

#[async_trait]
impl Request for BundleGraphRequest {
  #[tracing::instrument(skip_all)]
  async fn run(
    &self,
    _request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    // Base bundle graph state derived from the asset graph.
    // This primarily exists to assign public IDs to assets in a stable way.
    let bundle_graph = NativeBundleGraph::from_asset_graph(&self.asset_graph);

    // Find the entry dependency + entry asset for the main bundle.
    let mut entry_dep: Option<atlaspack_core::types::Dependency> = None;
    let mut entry_asset_id: Option<String> = None;

    for node in self.asset_graph.nodes() {
      if let AssetGraphNode::Dependency(dep) = node {
        if dep.is_entry {
          entry_dep = Some((**dep).clone());
          // Resolve to first outgoing asset
          if let Some(dep_node_id) = self.asset_graph.get_node_id_by_content_key(&dep.id) {
            let neighbors = self.asset_graph.get_outgoing_neighbors(dep_node_id);
            let mut resolved_asset: Option<String> = None;
            for n in neighbors {
              if let Some(AssetGraphNode::Asset(asset)) = self.asset_graph.get_node(&n) {
                resolved_asset = Some(asset.id.clone());
                break;
              }
            }
            entry_asset_id = resolved_asset;
          }
          break;
        }
      }
    }

    let entry_dep =
      entry_dep.ok_or_else(|| anyhow::anyhow!("BundleGraphRequest: missing entry dependency"))?;

    let entry_asset_id = entry_asset_id.ok_or_else(|| {
      anyhow::anyhow!("BundleGraphRequest: entry dependency did not resolve to an asset")
    })?;

    let target: Target = entry_dep.target.as_deref().cloned().unwrap_or_default();

    // Build a single monolithic JS bundle.
    let bundle_id = hash_string(format!(
      "bundle:{}{}",
      entry_asset_id,
      target.dist_dir.display()
    ));

    let bundle_public_id = generate_public_id(&bundle_id, |_candidate| false);

    let bundle = Bundle {
      id: bundle_id.clone(),
      public_id: Some(bundle_public_id),
      hash_reference: bundle_id.clone(),
      bundle_type: FileType::Js,
      env: (*target.env).clone(),
      entry_asset_ids: vec![entry_asset_id.clone()],
      main_entry_id: Some(entry_asset_id.clone()),
      needs_stable_name: Some(entry_dep.needs_stable_name),
      bundle_behavior: None,
      is_splittable: Some(false),
      manual_shared_bundle: None,
      name: None,
      pipeline: None,
      target: target.clone(),
    };

    // Bundle group id must match JS getBundleGroupId(bundleGroup)
    let bundle_group_id = format!("bundle_group:{}{}", target.name, entry_asset_id);

    // Nodes to add on the JS side (bundle_group + bundle)
    let bundle_group_node = serde_json::json!({
      "id": bundle_group_id,
      "type": "bundle_group",
      "value": {
        "target": target,
        "entryAssetId": entry_asset_id,
      }
    });

    let bundle_node = serde_json::json!({
      "id": bundle.id,
      "type": "bundle",
      "value": bundle,
    });

    // Compute bundle membership by traversing the asset graph from the entry asset.
    let mut stack: Vec<String> = vec![entry_asset_id.clone()];
    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();

    let asset_nodes_len = self.asset_graph.nodes().count() as u32;
    let bundle_group_node_id = asset_nodes_len;
    let bundle_node_id = asset_nodes_len + 1;

    let mut bundle_edges: Vec<u32> = Vec::new();
    let mut push_edge = |from: u32, to: u32, edge_type: u8| {
      bundle_edges.push(from);
      bundle_edges.push(to);
      bundle_edges.push(edge_type as u32);
    };

    let root_node_id = *self
      .asset_graph
      .get_node_id_by_content_key("@@root")
      .unwrap_or(&0) as u32;

    let entry_dep_node_id = *self
      .asset_graph
      .get_node_id_by_content_key(&entry_dep.id)
      .ok_or_else(|| anyhow::anyhow!("Missing entry dependency node id"))? as u32;

    let entry_asset_node_id = *self
      .asset_graph
      .get_node_id_by_content_key(&entry_asset_id)
      .ok_or_else(|| anyhow::anyhow!("Missing entry asset node id"))? as u32;

    // root -> bundle_group (bundle edge)
    push_edge(root_node_id, bundle_group_node_id, 3);
    // entry dependency -> bundle_group (null traversal)
    push_edge(entry_dep_node_id, bundle_group_node_id, 1);
    // bundle_group -> bundle (null + bundle)
    push_edge(bundle_group_node_id, bundle_node_id, 1);
    push_edge(bundle_group_node_id, bundle_node_id, 3);

    // bundle -> entry asset (null traversal)
    push_edge(bundle_node_id, entry_asset_node_id, 1);

    while let Some(asset_id) = stack.pop() {
      if !visited.insert(asset_id.clone()) {
        continue;
      }

      if let Some(node_id) = self.asset_graph.get_node_id_by_content_key(&asset_id) {
        let node_id_u32 = *node_id as u32;

        if let Some(AssetGraphNode::Asset(asset)) = self.asset_graph.get_node(node_id) {
          if asset.file_type == FileType::Js {
            // contains edge (2) + traversal edge (1)
            push_edge(bundle_node_id, node_id_u32, 2);
            push_edge(bundle_node_id, node_id_u32, 1);
          }
        }

        // Traverse downstream
        for child in self.asset_graph.get_outgoing_neighbors(node_id) {
          if let Some(child_node) = self.asset_graph.get_node(&child) {
            match child_node {
              AssetGraphNode::Dependency(dep) => {
                let dep_node_id = *self
                  .asset_graph
                  .get_node_id_by_content_key(&dep.id)
                  .unwrap() as u32;
                push_edge(bundle_node_id, dep_node_id, 2);
                // Continue traversal to resolved assets.
                for dep_child in self.asset_graph.get_outgoing_neighbors(&child) {
                  if let Some(AssetGraphNode::Asset(a)) = self.asset_graph.get_node(&dep_child) {
                    stack.push(a.id.clone());
                  }
                }
              }
              AssetGraphNode::Asset(a) => {
                stack.push(a.id.clone());
              }
              _ => {}
            }
          }
        }
      }
    }

    // Add a references edge from entry dependency -> entry asset (JS will remove the null edge).
    push_edge(entry_dep_node_id, entry_asset_node_id, 4);

    let mut output = BundleGraphRequestOutput {
      bundle_nodes: vec![bundle_group_node.to_string(), bundle_node.to_string()],
      bundle_edges,
      ..BundleGraphRequestOutput::from_bundle_graph(&bundle_graph)
    };

    // Ensure public id fields come from the NativeBundleGraph.
    output.public_id_by_asset_id = bundle_graph.public_id_by_asset_id.clone();
    output.asset_public_ids = bundle_graph.asset_public_ids.iter().cloned().collect();

    Ok(ResultAndInvalidations {
      result: RequestResult::BundleGraph(output),
      invalidations: vec![],
    })
  }
}
