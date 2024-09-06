use std::hash::Hash;
use std::path::PathBuf;
use std::sync::Arc;

use atlaspack_core::plugin::AssetBuildEvent;
use atlaspack_core::plugin::BuildProgressEvent;
use atlaspack_core::plugin::ReporterEvent;
use atlaspack_core::plugin::TransformResult;
use atlaspack_core::types::Asset;
use atlaspack_core::types::AssetStats;
use atlaspack_core::types::Dependency;
use atlaspack_core::types::Environment;

use crate::plugins::PluginsRef;
use crate::plugins::TransformerPipeline;
use crate::request_tracker::{Request, ResultAndInvalidations, RunRequestContext, RunRequestError};

use super::RequestResult;

/// The AssetRequest runs transformer plugins on discovered Assets.
/// - Decides which transformer pipeline to run from the input Asset type
/// - Runs the pipeline in series, switching pipeline if the Asset type changes
/// - Stores the final Asset source code in the cache, for access in packaging
/// - Finally, returns the complete Asset and it's discovered Dependencies
#[derive(Clone, Debug, Hash, PartialEq)]
pub struct AssetRequest {
  pub code: Option<String>,
  pub env: Arc<Environment>,
  pub file_path: PathBuf,
  pub pipeline: Option<String>,
  pub query: Option<String>,
  pub side_effects: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssetRequestOutput {
  pub asset: Asset,
  pub dependencies: Vec<Dependency>,
}

impl Request for AssetRequest {
  fn run(
    &self,
    request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    request_context.report(ReporterEvent::BuildProgress(BuildProgressEvent::Building(
      AssetBuildEvent {
        // TODO: Should we try avoid a clone here?
        file_path: self.file_path.clone(),
      },
    )));

    let pipeline = request_context
      .plugins()
      .transformers(&self.file_path, self.pipeline.clone())?;

    let asset = Asset::new(
      self.env.clone(),
      self.file_path.clone(),
      self.code.clone(),
      self.pipeline.clone(),
      self.side_effects,
      self.query.clone(),
      request_context.file_system().clone(),
    )?;

    let result = run_pipeline(pipeline, asset, request_context.plugins().clone())?;

    Ok(ResultAndInvalidations {
      result: RequestResult::Asset(AssetRequestOutput {
        asset: Asset {
          stats: AssetStats {
            size: result.asset.code.size(),
            time: 0,
          },
          ..result.asset
        },
        dependencies: result.dependencies,
      }),
      // TODO: Support invalidations
      invalidations: vec![],
    })
  }
}

fn run_pipeline(
  mut pipeline: TransformerPipeline,
  input: Asset,
  plugins: PluginsRef,
) -> anyhow::Result<TransformResult> {
  let mut dependencies = vec![];
  let mut invalidations = vec![];

  let mut transform_input = input;
  let original_asset_type = transform_input.file_type.clone();

  let pipeline_hash = pipeline.hash();
  for transformer in &mut pipeline.transformers {
    let transform_result = transformer.transform(transform_input)?;
    let is_different_asset_type = transform_result.asset.file_type != original_asset_type;

    transform_input = transform_result.asset;

    // If the Asset has changed type then we may need to trigger a different pipeline
    if is_different_asset_type {
      let next_pipeline = plugins.transformers(&transform_input.file_path, None)?;

      if next_pipeline.hash() != pipeline_hash {
        return run_pipeline(next_pipeline, transform_input, plugins);
      };
    }

    dependencies.extend(transform_result.dependencies);
    invalidations.extend(transform_result.invalidate_on_file_change);
  }

  Ok(TransformResult {
    asset: transform_input,
    dependencies,
    invalidate_on_file_change: invalidations,
  })
}
