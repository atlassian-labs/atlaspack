use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use async_trait::async_trait;
use serde::Serialize;

use atlaspack_core::asset_graph::AssetGraph;
use atlaspack_core::bundle_graph::NativeBundleGraph;

use crate::request_tracker::{Request, ResultAndInvalidations, RunRequestContext, RunRequestError};

use super::RequestResult;

/// The BundleGraphRequest is responsible for building the BundleGraph from the AssetGraph.
///
/// This creates the bundle graph structure from the asset graph, assigning public IDs
/// to assets and copying the graph structure. The actual bundling algorithm (creating
/// bundles and bundle groups) will be implemented in future milestones.
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

/// Output of the BundleGraphRequest
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleGraphRequestOutput {
  /// The number of nodes in the bundle graph
  pub node_count: usize,
  /// Edges between bundle graph nodes as (from, to) pairs
  pub edges: Vec<(u32, u32)>,
  /// Maps full asset IDs to concise public IDs
  pub public_id_by_asset_id: HashMap<String, String>,
  /// Set of all assigned asset public IDs
  pub asset_public_ids: Vec<String>,
}

impl BundleGraphRequestOutput {
  /// Create output from a BundleGraph
  pub fn from_bundle_graph(bundle_graph: &NativeBundleGraph) -> Self {
    Self {
      node_count: bundle_graph.node_count,
      edges: bundle_graph.edges.clone(),
      public_id_by_asset_id: bundle_graph.public_id_by_asset_id.clone(),
      asset_public_ids: bundle_graph.asset_public_ids.iter().cloned().collect(),
    }
  }
}

#[async_trait]
impl Request for BundleGraphRequest {
  async fn run(
    &self,
    _request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    // Create the bundle graph from the asset graph
    // This copies all nodes and edges, and assigns public IDs to assets
    let bundle_graph = NativeBundleGraph::from_asset_graph(&self.asset_graph);

    tracing::info!(
      "Created BundleGraph with {} nodes and {} edges",
      bundle_graph.node_count,
      bundle_graph.edges.len()
    );

    // Create the output
    let output = BundleGraphRequestOutput::from_bundle_graph(&bundle_graph);

    Ok(ResultAndInvalidations {
      result: RequestResult::BundleGraph(output),
      invalidations: vec![],
    })
  }
}
