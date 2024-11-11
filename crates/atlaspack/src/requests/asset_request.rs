use anyhow::anyhow;
use async_trait::async_trait;
use atlaspack_core::plugin::AssetBuildEvent;
use atlaspack_core::plugin::BuildProgressEvent;
use atlaspack_core::plugin::ReporterEvent;
use atlaspack_core::plugin::TransformContext;
use atlaspack_core::plugin::TransformResult;
use atlaspack_core::types::AssetStats;
use atlaspack_core::types::AssetWithDependencies;
use atlaspack_core::types::Code;
use atlaspack_core::types::Dependency;
use atlaspack_core::types::Environment;
use atlaspack_core::types::FileType;
use atlaspack_core::types::{Asset, Invalidation};
use atlaspack_sourcemap::find_sourcemap_url;
use atlaspack_sourcemap::load_sourcemap_url;
use std::collections::VecDeque;
use std::hash::Hash;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use crate::plugins::PluginsRef;
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
  pub discovered_assets: Vec<AssetWithDependencies>,
  pub dependencies: Vec<Dependency>,
}

#[async_trait]
impl Request for AssetRequest {
  async fn run(
    &self,
    request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    request_context
      .report(ReporterEvent::BuildProgress(BuildProgressEvent::Building(
        AssetBuildEvent {
          file_path: self.file_path.clone(),
        },
      )))
      .await;

    let start = Instant::now();

    let mut asset = Asset::new(
      self.env.clone(),
      self.file_path.clone(),
      request_context.file_system().clone(),
      self.pipeline.clone(),
      &self.project_root,
      self.code.clone(),
      self.side_effects,
      self.query.clone(),
    )?;

    // Load an existing sourcemap if available for valid file types
    if matches!(
      asset.file_type,
      FileType::Css | FileType::Js | FileType::Jsx | FileType::Ts | FileType::Tsx,
    ) {
      let code = asset.code.as_str()?;

      if let Some(url_match) = find_sourcemap_url(code) {
        // Sourcemaps are intentionally skipped if they are invalid
        if let Ok(source_map) = load_sourcemap_url(
          request_context.file_system(),
          &self.project_root,
          &asset.file_path,
          &url_match.url,
        ) {
          asset.map = Some(source_map);
          asset.code = Code::from(code.replace(&url_match.code, ""));
        }
      }
    }

    let transform_context = TransformContext::new(self.env.clone());
    let mut result =
      run_pipelines(transform_context, asset, request_context.plugins().clone()).await?;

    result.asset.stats = AssetStats {
      size: result.asset.code.size(),
      time: start.elapsed().as_millis().try_into().unwrap_or(u32::MAX),
    };
    result.asset.code = "".into();

    Ok(ResultAndInvalidations::new(
      RequestResult::Asset(AssetRequestOutput {
        asset: result.asset,
        // TODO: Need to decide whether a discovered asset will belong to the asset graph as it's own node
        discovered_assets: result.discovered_assets,
        dependencies: result.dependencies,
      }),
      result
        .invalidate_on_file_change
        .into_iter()
        .map(Invalidation::FileChange)
        .collect(),
    ))
  }
}

async fn run_pipelines(
  transform_context: TransformContext,
  input: Asset,
  plugins: PluginsRef,
) -> anyhow::Result<TransformResult> {
  let mut invalidations = vec![];
  let mut asset_queue = VecDeque::from([(
    AssetWithDependencies {
      asset: input,
      dependencies: Vec::new(),
    },
    None,
  )]);
  let mut initial_asset: Option<Asset> = None;
  let mut initial_asset_dependencies = None;
  let mut processed_assets: Vec<AssetWithDependencies> = vec![];

  while let Some((
    AssetWithDependencies {
      asset: asset_to_modify,
      dependencies,
    },
    pipeline,
  )) = asset_queue.pop_front()
  {
    let original_asset_type = asset_to_modify.file_type.clone();
    let (mut pipeline, pipeline_id) = if let Some(pipeline_info) = pipeline {
      pipeline_info
    } else {
      let mut file_path = asset_to_modify.file_path.clone();
      file_path.set_extension(asset_to_modify.file_type.extension());

      let pipeline = plugins.transformers(&file_path, asset_to_modify.pipeline.clone())?;
      let pipeline_id = pipeline.id();

      (pipeline, pipeline_id)
    };

    let mut current_asset = asset_to_modify.clone();
    let mut current_dependencies = dependencies;

    for transformer in pipeline.transformers_mut() {
      let transform_result = transformer
        .transform(transform_context.clone(), current_asset)
        .await?;

      current_asset = transform_result.asset;

      current_dependencies.extend(transform_result.dependencies);
      invalidations.extend(transform_result.invalidate_on_file_change);
      asset_queue.extend(
        transform_result
          .discovered_assets
          .into_iter()
          .map(|discovered_asset| (discovered_asset, None)),
      );

      // If the Asset has changed type then we may need to trigger a different pipeline
      if current_asset.file_type != original_asset_type {
        let next_pipeline =
          plugins.transformers(&current_asset.file_path, current_asset.pipeline.clone())?;

        let next_pipeline_id = next_pipeline.id();

        if next_pipeline_id != pipeline_id {
          asset_queue.push_front((
            AssetWithDependencies {
              asset: current_asset.clone(),
              dependencies: current_dependencies.clone(),
            },
            Some((next_pipeline, next_pipeline_id)),
          ));
          break;
        }
      }
    }

    // We assume the first asset to complete the pipeline is the initial asset
    if initial_asset.is_none() {
      initial_asset = Some(current_asset);
      initial_asset_dependencies = Some(current_dependencies);
    } else {
      processed_assets.push(AssetWithDependencies {
        asset: current_asset,
        dependencies: current_dependencies,
      });
    }
  }

  Ok(TransformResult {
    asset: initial_asset
      .ok_or_else(|| anyhow!("Initial asset missing after transformer pipeline"))?,
    dependencies: initial_asset_dependencies
      .ok_or_else(|| anyhow!("Initial asset missing after transformer pipeline"))?,
    discovered_assets: processed_assets,
    invalidate_on_file_change: invalidations,
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::plugins::MockPlugins;
  use crate::plugins::TransformerPipeline;
  use atlaspack_core::hash::IdentifierHasher;
  use atlaspack_core::plugin::MockTransformerPlugin;
  use atlaspack_core::types::Code;
  use atlaspack_core::types::FileType;
  use std::hash::Hasher;
  use std::path::Path;

  fn make_asset(file_path: &str, file_type: FileType) -> Asset {
    let mut asset = Asset::default();

    asset.file_path = PathBuf::from(file_path);
    asset.file_type = file_type;

    asset
  }

  fn assert_code(asset: &Asset, code: &str) {
    assert_eq!(
      String::from_utf8(asset.code.bytes().to_vec()).unwrap(),
      code
    )
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_run_pipelines_works() {
    let mut plugins = MockPlugins::new();
    plugins.expect_transformers().returning(move |_, _| {
      Ok(TransformerPipeline::new(vec![
        make_transformer("transformer1", None, None, None),
        make_transformer("transformer2", None, None, None),
      ]))
    });

    let asset = Asset::default();
    let context = TransformContext::default();
    let result = run_pipelines(context, asset, Arc::new(plugins))
      .await
      .unwrap();

    assert_code(&result.asset, "::transformer1::transformer2");
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_run_pipelines_with_invalidations() {
    let mut plugins = MockPlugins::new();
    plugins
      .expect_transformers()
      .withf(|path: &Path, _pipeline: &Option<String>| {
        path.extension().is_some_and(|ext| ext == "js")
      })
      .returning(move |_, _| {
        Ok(TransformerPipeline::new(vec![
          make_transformer("js-1", None, None, None),
          make_transformer("js-2", None, None, Some(vec![PathBuf::from("./tmp")])),
        ]))
      });

    let asset = make_asset("index.js", FileType::Js);
    let expected_invalidations = vec![PathBuf::from("./tmp")];
    let context = TransformContext::default();
    let result = run_pipelines(context, asset, Arc::new(plugins))
      .await
      .unwrap();

    assert_code(&result.asset, "::js-1::js-2");
    assert_eq!(result.invalidate_on_file_change, expected_invalidations);
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_run_pipelines_with_dependencies() {
    let mut plugins = MockPlugins::new();
    plugins
      .expect_transformers()
      .withf(|path: &Path, _pipeline: &Option<String>| {
        path.extension().is_some_and(|ext| ext == "js")
      })
      .returning(move |_, _| {
        Ok(TransformerPipeline::new(vec![
          make_transformer("js-1", None, None, None),
          make_transformer("js-2", None, Some(vec![Dependency::default()]), None),
        ]))
      });

    let asset = make_asset("index.js", FileType::Js);
    let expected_dependencies = vec![Dependency::default()];
    let context = TransformContext::default();
    let result = run_pipelines(context, asset, Arc::new(plugins))
      .await
      .unwrap();

    assert_code(&result.asset, "::js-1::js-2");
    assert_eq!(result.dependencies, expected_dependencies);
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_run_pipelines_with_discovered_assets() {
    let mut plugins = MockPlugins::new();

    plugins
      .expect_transformers()
      .withf(|path: &Path, _pipeline: &Option<String>| {
        path.extension().is_some_and(|ext| ext == "js")
      })
      .returning(move |_, _| {
        Ok(TransformerPipeline::new(vec![
          make_transformer("js-1", None, None, None),
          make_transformer(
            "js-2",
            Some(vec![AssetWithDependencies {
              asset: make_asset("discovered.css", FileType::Css),
              dependencies: vec![Dependency::default()],
            }]),
            None,
            None,
          ),
        ]))
      });

    plugins
      .expect_transformers()
      .withf(|path: &Path, _pipeline: &Option<String>| {
        path.extension().is_some_and(|ext| ext == "css")
      })
      .returning(move |_, _| {
        Ok(TransformerPipeline::new(vec![
          make_transformer("css-1", None, None, None),
          make_transformer("css-2", None, Some(vec![Dependency::default()]), None),
        ]))
      });

    let asset = make_asset("index.js", FileType::Js);
    let context = TransformContext::default();
    let result = run_pipelines(context, asset, Arc::new(plugins))
      .await
      .unwrap();

    assert_code(&result.asset, "::js-1::js-2");
    assert_eq!(result.discovered_assets.len(), 1);
    assert_code(&result.discovered_assets[0].asset, "::css-1::css-2");
    assert_eq!(result.discovered_assets[0].dependencies.len(), 2);
  }

  fn make_transformer(
    label: &str,
    discovered_assets: Option<Vec<AssetWithDependencies>>,
    dependencies: Option<Vec<Dependency>>,
    invalidate_on_file_change: Option<Vec<PathBuf>>,
  ) -> Arc<MockTransformerPlugin> {
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

    mock.expect_transform().returning({
      let label = label.clone();
      move |_context, asset: Asset| {
        let mut asset = asset.clone();
        asset.code = Code::from(format!(
          "{}::{}",
          String::from_utf8(asset.code.bytes().to_vec()).unwrap(),
          label.clone()
        ));

        Ok(TransformResult {
          asset,
          discovered_assets: discovered_assets.clone().unwrap_or_default(),
          dependencies: dependencies.clone().unwrap_or_default(),
          invalidate_on_file_change: invalidate_on_file_change.clone().unwrap_or_default(),
        })
      }
    });
    // .returning(move |_context, asset: Asset| get_simple_transformer(label.clone(), asset));
    Arc::new(mock)
  }
}
