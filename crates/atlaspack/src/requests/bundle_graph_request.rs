use std::hash::{Hash, Hasher};
use std::sync::Arc;

use async_trait::async_trait;
use atlaspack_core::asset_graph::AssetGraph;
use atlaspack_core::bundle_graph::NativeBundleGraph;

use atlaspack_bundling::{Bundler, MonolithicBundler};

use crate::request_tracker::{Request, ResultAndInvalidations, RunRequestContext, RunRequestError};

use super::RequestResult;

#[derive(Debug, Default)]
pub struct BundleGraphRequest {
  pub asset_graph: Arc<AssetGraph>,
}

impl Hash for BundleGraphRequest {
  fn hash<H: Hasher>(&self, _state: &mut H) {}
}

#[derive(Clone, Debug, PartialEq)]
pub struct BundleGraphRequestOutput {
  pub bundle_graph: NativeBundleGraph,
  pub had_previous_graph: bool,
}

#[async_trait]
impl Request for BundleGraphRequest {
  #[tracing::instrument(skip_all)]
  async fn run(
    &self,
    _request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    let mut bundle_graph = NativeBundleGraph::from_asset_graph(&self.asset_graph);

    let bundler = MonolithicBundler;
    bundler.bundle(&self.asset_graph, &mut bundle_graph)?;

    let output = BundleGraphRequestOutput {
      bundle_graph,
      had_previous_graph: false,
    };

    Ok(ResultAndInvalidations {
      result: RequestResult::BundleGraph(output),
      invalidations: vec![],
    })
  }
}
