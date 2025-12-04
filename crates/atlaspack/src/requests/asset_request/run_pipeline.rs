use atlaspack_core::plugin::TransformContext;
use atlaspack_core::types::Asset;
use atlaspack_core::types::AssetWithDependencies;
use atlaspack_core::types::Dependency;
use atlaspack_memoization_cache::CacheResponse;
use atlaspack_memoization_cache::Cacheable;
use serde::Deserialize;
use serde::Serialize;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::PathBuf;

use crate::plugins::PluginsRef;
use crate::plugins::TransformerPipeline;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum PipelineResult {
  Complete(Asset, Vec<Dependency>),
  TypeChange(Asset, Vec<Dependency>),
}

pub struct RunPipelineInput {
  pub asset: Asset,
  pub pipeline: TransformerPipeline,
  pub context: TransformContext,
  pub plugins: PluginsRef,
}

impl Cacheable for RunPipelineInput {
  fn cache_key(&self) -> Option<(String, String)> {
    let mut hasher = atlaspack_core::hash::IdentifierHasher::default();
    // Using asset id for now
    self.asset.id.hash(&mut hasher);

    // If the pipeline has no cache key then it is uncachable
    self.pipeline.cache_key?.hash(&mut hasher);

    // Ignore context and plugins from the cache key

    let label = format!(
      "run_pipeline|{}",
      self
        .asset
        .file_path
        .strip_prefix(&self.context.config().project_root)
        .unwrap_or(&self.asset.file_path)
        .display()
    );
    let cache_key = format!("{}|{}", &label, hasher.finish());

    Some((label, cache_key))
  }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RunPipelineOutput {
  pub invalidations: Vec<PathBuf>,
  pub discovered_assets: Vec<AssetWithDependencies>,
  pub pipeline_result: PipelineResult,
  cache_bailout: bool,
}

impl CacheResponse for RunPipelineOutput {
  fn bailout(&self) -> bool {
    self.cache_bailout
  }
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn run_pipeline(input: RunPipelineInput) -> anyhow::Result<RunPipelineOutput> {
  let RunPipelineInput {
    asset,
    pipeline,
    context,
    plugins,
  } = input;

  let mut current_asset = asset.clone();
  let mut dependencies = Vec::new();
  let mut discovered_assets = Vec::new();
  let mut invalidations = Vec::new();

  let original_asset_type = current_asset.file_type.clone();

  let mut cache_bailout = false;

  for transformer in pipeline.transformers() {
    let transform_result = transformer
      .transform(context.clone(), current_asset)
      .await?;

    if transform_result.cache_bailout {
      cache_bailout = true;
    }

    current_asset = transform_result.asset;

    dependencies.extend(transform_result.dependencies);
    invalidations.extend(transform_result.invalidate_on_file_change);
    discovered_assets.extend(transform_result.discovered_assets);

    // If the Asset has changed type then we may need to trigger a different pipeline
    if current_asset.file_type != original_asset_type {
      // When the Asset changes file_type we need to regenerate its id
      current_asset.update_id(&context.config().project_root);

      let next_pipeline = plugins.transformers(&current_asset).await?;

      if pipeline.should_run_new_pipeline(&next_pipeline) {
        tracing::debug!(
          "Asset requires pipeline switch. File: {} from {} to {}",
          current_asset.file_path.display(),
          original_asset_type.extension(),
          current_asset.file_type.extension()
        );

        return Ok(RunPipelineOutput {
          invalidations,
          discovered_assets,
          pipeline_result: PipelineResult::TypeChange(current_asset, dependencies),
          cache_bailout,
        });
      }
    }
  }

  Ok(RunPipelineOutput {
    invalidations,
    discovered_assets,
    pipeline_result: PipelineResult::Complete(current_asset, dependencies),
    cache_bailout,
  })
}
