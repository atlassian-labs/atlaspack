use async_trait::async_trait;

use crate::request_tracker::{Request, ResultAndInvalidations, RunRequestContext, RunRequestError};

use super::{AssetGraphRequest, BundleGraphRequest, BundleGraphRequestOutput, RequestResult};

/// Output of the full native build pipeline.
#[derive(Clone, Debug, PartialEq)]
pub struct BuildRequestOutput {
  pub bundle_graph: BundleGraphRequestOutput,
}

/// Top-level request that composes the full build pipeline:
/// 1. Build asset graph
/// 2. Build bundle graph
/// 3. Package and write bundles (stub — will be filled in by PackagingRequest)
#[derive(Debug, Hash)]
pub struct BuildRequest {}

#[async_trait]
impl Request for BuildRequest {
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

    // 2. Build bundle graph
    let (bundle_graph_result, _, _) = request_context
      .execute_request(BundleGraphRequest { asset_graph })
      .await?;

    let RequestResult::BundleGraph(bundle_graph_output) = bundle_graph_result.as_ref() else {
      anyhow::bail!("Unexpected request result from BundleGraphRequest");
    };

    // 3. Package and write bundles — stub for now
    // This will be replaced by PackagingRequest once it is implemented.
    // For now, the build pipeline stops after bundling.
    tracing::info!("BuildRequest: packaging step is a stub — skipping");

    Ok(ResultAndInvalidations {
      result: RequestResult::Build(BuildRequestOutput {
        bundle_graph: bundle_graph_output.clone(),
      }),
      invalidations: Vec::new(),
    })
  }
}
