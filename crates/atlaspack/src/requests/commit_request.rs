use std::hash::{Hash, Hasher};
use std::sync::Arc;

use async_trait::async_trait;
use atlaspack_core::asset_graph::{AssetGraph, AssetGraphNode};

use crate::request_tracker::{Request, ResultAndInvalidations, RunRequestContext, RunRequestError};

use super::RequestResult;

/// Output of the commit request.
#[derive(Clone, Debug, PartialEq)]
pub struct CommitRequestOutput {
  /// Number of assets whose content was written to the database.
  pub committed_count: usize,
}

/// Writes transformed asset content and source maps to the database so that
/// downstream requests (e.g. packaging) can read them back by asset ID.
///
/// This mirrors the JS-side asset commit step that runs between the asset graph
/// build and packaging. It iterates over new and updated asset nodes and stores:
/// - Asset compiled code under the asset's ID
/// - Source maps (if present) under `map:{asset_id}`
#[derive(Debug)]
pub struct CommitRequest {
  pub asset_graph: Arc<AssetGraph>,
}

impl Hash for CommitRequest {
  fn hash<H: Hasher>(&self, state: &mut H) {
    // Hash based on the number of nodes — if the graph changes, the request
    // is re-run. This is a coarse key; the request tracker's invalidation
    // system handles fine-grained cache busting via file-change edges.
    self.asset_graph.nodes().count().hash(state);
  }
}

#[async_trait]
impl Request for CommitRequest {
  fn request_type(&self) -> &'static str {
    "CommitRequest"
  }
  #[tracing::instrument(level = "info", skip_all)]
  async fn run(
    &self,
    request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    let db = &request_context.db;
    let mut committed_count: usize = 0;

    let nodes = self
      .asset_graph
      .new_nodes()
      .chain(self.asset_graph.updated_nodes());

    for node in nodes {
      let AssetGraphNode::Asset(asset) = node else {
        continue;
      };

      // Use content_key if the asset has one (set by the JS side during transformation),
      // otherwise fall back to asset.id for natively-built assets.
      let key = asset.content_key.as_deref().unwrap_or(&asset.id);
      db.put(key, asset.code.bytes())?;

      if let Some(map) = &asset.map {
        db.put(&format!("map:{key}"), map.clone().to_json(None)?.as_bytes())?;
      }

      committed_count += 1;
    }

    Ok(ResultAndInvalidations {
      result: RequestResult::Commit(CommitRequestOutput { committed_count }),
      invalidations: vec![],
    })
  }
}

#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use atlaspack_core::database::{Database, DatabaseRef, InMemoryDatabase};
  use atlaspack_core::types::{Asset, Code};
  use pretty_assertions::assert_eq;

  use crate::requests::RequestResult;
  use crate::test_utils::{RequestTrackerTestOptions, request_tracker_with_db};

  use super::*;

  async fn run_commit_request(db: DatabaseRef, asset_graph: AssetGraph) -> CommitRequestOutput {
    let mut rt = request_tracker_with_db(RequestTrackerTestOptions::default(), Arc::clone(&db));
    let request = CommitRequest {
      asset_graph: Arc::new(asset_graph),
    };
    let result = rt.run_request(request).await.unwrap();
    match result.as_ref() {
      RequestResult::Commit(output) => output.clone(),
      other => panic!("Expected Commit result, got {other}"),
    }
  }

  #[test]
  fn commits_new_asset_content_to_database() {
    let db: DatabaseRef = Arc::new(InMemoryDatabase::default());

    let mut asset_graph = AssetGraph::new();
    asset_graph.add_asset(
      Arc::new(Asset {
        id: String::from("asset-1"),
        code: Code::from(String::from("console.log('hello');")),
        ..Asset::default()
      }),
      false,
    );
    asset_graph.add_asset(
      Arc::new(Asset {
        id: String::from("asset-2"),
        code: Code::from(String::from("export default 42;")),
        ..Asset::default()
      }),
      false,
    );

    let output = tokio::runtime::Runtime::new()
      .unwrap()
      .block_on(run_commit_request(Arc::clone(&db), asset_graph));

    assert_eq!(output.committed_count, 2);

    // Verify the data was written to the in-memory database
    assert_eq!(
      db.get("asset-1").unwrap(),
      Some(b"console.log('hello');".to_vec())
    );
    assert_eq!(
      db.get("asset-2").unwrap(),
      Some(b"export default 42;".to_vec())
    );
  }

  #[test]
  fn skips_non_asset_nodes() {
    let db: DatabaseRef = Arc::new(InMemoryDatabase::default());

    // An empty asset graph has no asset nodes (only the root dependency node)
    let asset_graph = AssetGraph::new();

    let output = tokio::runtime::Runtime::new()
      .unwrap()
      .block_on(run_commit_request(Arc::clone(&db), asset_graph));

    assert_eq!(output.committed_count, 0);
  }
}
