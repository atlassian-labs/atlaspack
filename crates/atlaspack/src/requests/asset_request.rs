use atlaspack_core::plugin::AssetBuildEvent;
use atlaspack_core::plugin::BuildProgressEvent;
use atlaspack_core::plugin::ReporterEvent;
use atlaspack_core::plugin::TransformContext;
use atlaspack_core::plugin::TransformResult;
use atlaspack_core::types::AssetStats;
use atlaspack_core::types::Dependency;
use atlaspack_core::types::Environment;
use atlaspack_core::types::{Asset, Invalidation};
use std::collections::VecDeque;
use std::hash::Hash;
use std::path::PathBuf;
use std::sync::Arc;

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
  pub project_root: PathBuf,
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
  pub discovered_assets: Vec<Asset>, // TODO: model this better
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
      &self.project_root,
      self.env.clone(),
      self.file_path.clone(),
      self.code.clone(),
      self.pipeline.clone(),
      self.side_effects,
      self.query.clone(),
      request_context.file_system().clone(),
    )?;

    let transform_context = TransformContext::new(self.env.clone());
    let mut result = run_pipeline(
      transform_context,
      pipeline,
      asset,
      request_context.plugins().clone(),
    )?;

    result.asset.stats = AssetStats {
      size: result.asset.code.size(),
      time: 0,
    };

    Ok(ResultAndInvalidations {
      result: RequestResult::Asset(AssetRequestOutput {
        asset: result.asset,
        // TODO: Need to decide whether a discovered asset will belong to the asset graph as it's own node
        discovered_assets: result.discovered_assets,
        dependencies: result.dependencies,
      }),
      invalidations: result
        .invalidate_on_file_change
        .into_iter()
        .map(|path| Invalidation::FileChange(path))
        .collect(),
    })
  }
}

pub fn run_pipeline(
  transform_context: TransformContext,
  mut pipeline: TransformerPipeline,
  input: Asset,
  plugins: PluginsRef,
) -> anyhow::Result<TransformResult> {
  let mut dependencies = vec![];
  let mut invalidations = vec![];
  let mut asset_queue = VecDeque::from([input]);
  let mut initial_asset: Option<Asset> = None;
  let mut processed_assets: Vec<Asset> = vec![];

  while let Some(asset_to_modify) = asset_queue.pop_front() {
    let original_asset_type = asset_to_modify.file_type.clone();
    let pipeline_hash = pipeline.id();

    let mut current_asset = asset_to_modify.clone();

    for transformer in pipeline.transformers_mut() {
      let transform_result = transformer.transform(transform_context.clone(), current_asset)?;
      let is_different_asset_type = transform_result.asset.file_type != original_asset_type;
      current_asset = transform_result.asset;

      dependencies.extend(transform_result.dependencies);
      invalidations.extend(transform_result.invalidate_on_file_change);
      asset_queue.extend(transform_result.discovered_assets);
      // If the Asset has changed type then we may need to trigger a different pipeline
      if is_different_asset_type {
        let next_pipeline = plugins.transformers(&current_asset.file_path, None)?;
        if next_pipeline.id() != pipeline_hash {
          let clone = current_asset.clone();
          asset_queue.push_front(clone);
          pipeline = next_pipeline;
          break;
        }
      }
    }

    if initial_asset.is_none() {
      initial_asset = Some(current_asset);
    }
    processed_assets.push(asset_to_modify);
  }

  Ok(TransformResult {
    asset: initial_asset.unwrap(),
    discovered_assets: processed_assets,
    dependencies,
    invalidate_on_file_change: invalidations,
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::plugins::MockPlugins;
  use anyhow::Error;
  use atlaspack_core::hash::IdentifierHasher;
  use atlaspack_core::plugin::MockTransformerPlugin;
  use atlaspack_core::types::Code;
  use std::hash::Hasher;

  #[test]
  fn test_run_pipeline_works() {
    let mock_transformer1 = make_transformer("transformer1", None, None, None);
    let mock_transformer2 = make_transformer("transformer2", None, None, None);

    let pipeline = TransformerPipeline::new(vec![mock_transformer1, mock_transformer2]);

    let asset = Asset::default();
    let plugins = Arc::new(MockPlugins::new());
    let context = TransformContext::default();
    let result = run_pipeline(context, pipeline, asset, plugins).unwrap();

    assert_eq!(
      String::from_utf8(result.asset.code.bytes().to_vec()).unwrap(),
      "::transformer1::transformer2"
    )
  }
  #[test]
  fn test_run_pipeline_with_all_fields() {
    let mock_transformer = make_transformer(
      "transformer",
      Some(vec![Asset::default()]),
      Some(vec![Dependency::default()]),
      Some(vec![PathBuf::from("./tmp")]),
    );
    let pipeline = TransformerPipeline::new(vec![mock_transformer]);
    let asset = Asset::default();
    let plugins = Arc::new(MockPlugins::new());
    let expected_discovered_assets = vec![Asset::default()];
    let expected_dependencies = vec![Dependency::default()];
    let expected_invalidations = vec![PathBuf::from("./tmp")];
    let context = TransformContext::default();
    let result = run_pipeline(context, pipeline, asset, plugins).unwrap();
    assert_eq!(
      String::from_utf8(result.asset.code.bytes().to_vec()).unwrap(),
      "::transformer"
    );
    assert_eq!(result.discovered_assets, expected_discovered_assets);
    assert_eq!(result.dependencies, expected_dependencies);
    assert_eq!(result.invalidate_on_file_change, expected_invalidations);
  }

  fn make_transformer(
    label: &str,
    discovered_assets: Option<Vec<Asset>>,
    dependencies: Option<Vec<Dependency>>,
    invalidate_on_file_change: Option<Vec<PathBuf>>,
  ) -> Box<MockTransformerPlugin> {
    let label = label.to_string();
    let mut mock = MockTransformerPlugin::new();
    mock.expect_id().returning({
      let label = label.clone();
      move || {
        let mut hasher = IdentifierHasher::new();
        label.hash(&mut hasher);
        hasher.finish()
      }
    });

    mock
      .expect_transform()
      .return_once({
        let label = label.clone();
        move |_context, mut asset: Asset| {
          asset.code = Arc::new(Code::from(format!(
            "{}::{}",
            String::from_utf8(asset.code.bytes().to_vec()).unwrap(),
            label.clone()
          )));

          let result_discovered_assets: Vec<Asset> = if discovered_assets.is_some() {
            discovered_assets.unwrap()
          } else {
            vec![]
          };
          let result_dependencies = if dependencies.is_some() {
            dependencies.unwrap()
          } else {
            vec![]
          };
          let result_invalidations = if invalidate_on_file_change.is_some() {
            invalidate_on_file_change.unwrap()
          } else {
            vec![]
          };

          Ok(TransformResult {
            asset,
            discovered_assets: result_discovered_assets,
            dependencies: result_dependencies,
            invalidate_on_file_change: result_invalidations,
          })
        }
      })
      .returning(move |_context, asset: Asset| get_simple_transformer(label.clone(), asset));
    Box::new(mock)
  }

  fn get_simple_transformer(label: String, mut asset: Asset) -> Result<TransformResult, Error> {
    asset.code = Arc::new(Code::from(format!(
      "{}::{label}",
      String::from_utf8(asset.code.bytes().to_vec()).unwrap()
    )));
    // Set the fields to be checked in the test
    let discovered_assets = vec![];
    let dependencies = vec![Dependency::default()];
    let invalidations = vec![PathBuf::from("./tmp")];
    Ok(TransformResult {
      asset,
      discovered_assets,
      dependencies,
      invalidate_on_file_change: invalidations,
      ..TransformResult::default()
    })
  }
}
