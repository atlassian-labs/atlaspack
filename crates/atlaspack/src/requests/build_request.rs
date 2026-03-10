use async_trait::async_trait;
use atlaspack_core::build_progress::BuildProgressEvent;
use std::sync::Arc;

use crate::{
  request_tracker::{Request, ResultAndInvalidations, RunRequestContext, RunRequestError},
  requests::packaging_request::PackagingRequest,
};

use super::{
  AssetGraphRequest, BundleGraphRequest, BundleGraphRequestOutput, CommitRequest, RequestResult,
};
use crate::requests::packaging_request::PackagingRequestOutput;

/// Output of the full native build pipeline.
#[derive(Clone, Debug, PartialEq)]
pub struct BuildRequestOutput {
  pub bundle_graph: BundleGraphRequestOutput,
  pub packaging: PackagingRequestOutput,
}

/// Top-level request that composes the full build pipeline:
/// 1. Build asset graph
/// 2. Commit asset content to DB and build bundle graph (in parallel)
/// 3. Package and write bundles
#[derive(Debug, Hash)]
pub struct BuildRequest {}

#[async_trait]
impl Request for BuildRequest {
  fn request_type(&self) -> &'static str {
    "BuildRequest"
  }
  #[tracing::instrument(level = "info", skip_all)]
  async fn run(
    &self,
    mut request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    // 1. Build asset graph
    let (asset_graph_result, _, _) = request_context
      .execute_request(AssetGraphRequest {
        prev_asset_graph: None,
        incrementally_bundled_assets: None,
      })
      .await?;

    let RequestResult::AssetGraph(asset_graph_output) = asset_graph_result.as_ref() else {
      anyhow::bail!("Unexpected request result from AssetGraphRequest");
    };

    let asset_graph = asset_graph_output.graph.clone();

    // 2. Commit asset content to database and build bundle graph in parallel.
    //    Both depend on the asset graph but not on each other.
    let commit_future = request_context.execute_request(CommitRequest {
      asset_graph: Arc::clone(&asset_graph),
    });

    request_context.report(BuildProgressEvent::Bundling);
    let bundle_graph_future = request_context.execute_request(BundleGraphRequest {
      asset_graph: Arc::clone(&asset_graph),
    });

    // Await both — order doesn't matter, but commit must complete before packaging.
    let (commit_result, bundle_graph_result) =
      tokio::try_join!(commit_future, bundle_graph_future)?;

    let RequestResult::Commit(_) = commit_result.0.as_ref() else {
      anyhow::bail!("Unexpected request result from CommitRequest");
    };

    let RequestResult::BundleGraph(bundle_graph_output) = bundle_graph_result.0.as_ref() else {
      anyhow::bail!("Unexpected request result from BundleGraphRequest");
    };

    // 3. Package and write bundles (pass reference; packaging reads from the same graph)
    let (packaging_result, _, _) = request_context
      .execute_request(PackagingRequest::new(
        bundle_graph_output.bundle_graph.clone(),
      ))
      .await?;

    let RequestResult::Packaging(packaging_output) = packaging_result.as_ref() else {
      anyhow::bail!("Unexpected request result from PackagingRequest");
    };

    Ok(ResultAndInvalidations {
      result: RequestResult::Build(BuildRequestOutput {
        bundle_graph: bundle_graph_output.clone(),
        packaging: packaging_output.clone(),
      }),
      invalidations: Vec::new(),
    })
  }
}
