use std::hash::{Hash, Hasher};
use std::sync::Arc;

use async_trait::async_trait;
use atlaspack_core::asset_graph::{AssetGraph, AssetGraphNode};
use atlaspack_core::bundle_graph::NativeBundleGraph;

use atlaspack_bundling::{Bundler, IdealGraphBundler, MonolithicBundler};

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

/// Check whether all entry dependencies in the asset graph have
/// `unstable_single_file_output` set on their target environment.
///
/// This mirrors the logic in `DefaultBundler.ts` which separates entries into
/// `singleFileEntries` (MonolithicBundler) and `idealGraphEntries` (IdealGraphBundler).
fn should_use_monolithic_bundler(asset_graph: &AssetGraph) -> bool {
  let mut has_entries = false;

  for node in asset_graph.nodes() {
    let AssetGraphNode::Dependency(dep) = node else {
      continue;
    };
    if !dep.is_entry {
      continue;
    }
    has_entries = true;

    let is_single_file = dep
      .target
      .as_ref()
      .map(|t| t.env.unstable_single_file_output)
      .unwrap_or(false);

    if !is_single_file {
      return false;
    }
  }

  has_entries
}

#[async_trait]
impl Request for BundleGraphRequest {
  #[tracing::instrument(skip_all)]
  async fn run(
    &self,
    _request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    let mut bundle_graph = NativeBundleGraph::from_asset_graph(&self.asset_graph);

    if should_use_monolithic_bundler(&self.asset_graph) {
      let bundler = MonolithicBundler;
      bundler.bundle(&self.asset_graph, &mut bundle_graph)?;
    } else {
      let bundler = IdealGraphBundler::default();
      bundler.bundle(&self.asset_graph, &mut bundle_graph)?;
    }

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
