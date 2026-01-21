use anyhow::anyhow;
use async_trait::async_trait;
use atlaspack_core::hash::hash_bytes;
use atlaspack_core::plugin::TransformResult;
use atlaspack_core::plugin::TransformSymbolInfo;
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

use crate::request_tracker::{Request, ResultAndInvalidations, RunRequestContext, RunRequestError};
use crate::requests::asset_request::run_pipeline::PipelineResult;
use crate::requests::asset_request::run_pipeline::RunPipelineInput;
use crate::requests::asset_request::run_pipeline::RunPipelineOutput;
use crate::requests::asset_request::run_pipeline::run_pipeline;

use super::RequestResult;

mod run_pipeline;

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
  pub asset: Arc<Asset>,
  pub discovered_assets: Vec<AssetWithDependencies>,
  pub dependencies: Vec<Dependency>,
}

#[async_trait]
impl Request for AssetRequest {
  #[tracing::instrument(level = "trace", skip_all)]
  async fn run(
    &self,
    request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    let start = Instant::now();

    let code = if let Some(code) = self.code.as_ref() {
      Code::from(code.to_owned())
    } else {
      let code_from_disk = request_context.file_system().read(&self.file_path)?;
      Code::new(code_from_disk)
    };

    let mut asset = Asset::new(
      code,
      self.code.is_some(),
      self.env.clone(),
      self.file_path.clone(),
      self.pipeline.clone(),
      &self.project_root,
      self.query.clone(),
      self.side_effects,
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
          &self.file_path,
          &url_match.url,
        ) {
          asset.map = Some(source_map);
          asset.code = Code::from(code.replace(&url_match.code, ""));
        }
      }
    }

    let mut result = run_pipelines(asset, request_context).await?;

    // TODO: Commit the asset with a project path now, as transformers rely on an absolute path
    // result.asset.file_path = to_project_path(&self.project_root, &result.asset.file_path);

    result.asset.stats = AssetStats {
      size: result.asset.code.size(),
      time: start.elapsed().as_millis().try_into().unwrap_or(u32::MAX),
    };

    // Assign the output hash for the Asset now that all transforms have been
    // applied
    result.asset.output_hash = Some(hash_bytes(result.asset.code.bytes()));

    // Ensure the asset source file is marked as an invalidation
    result
      .invalidate_on_file_change
      .push(result.asset.file_path.clone());

    Ok(ResultAndInvalidations {
      result: RequestResult::Asset(AssetRequestOutput {
        asset: Arc::new(result.asset),
        // TODO: Need to decide whether a discovered asset will belong to the asset graph as it's own node
        discovered_assets: result.discovered_assets,
        dependencies: result.dependencies,
      }),
      invalidations: result
        .invalidate_on_file_change
        .into_iter()
        .map(Invalidation::FileChange)
        .collect(),
    })
  }
}

#[tracing::instrument(level = "trace", skip_all)]
async fn run_pipelines(
  input: Asset,
  request_context: RunRequestContext,
) -> anyhow::Result<TransformResult> {
  let plugins = request_context.plugins();
  let mut all_invalidations = vec![];
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
      dependencies: prev_dependencies,
    },
    pipeline,
  )) = asset_queue.pop_front()
  {
    let pipeline = if let Some(pipeline_info) = pipeline {
      pipeline_info
    } else {
      let mut file_path = asset_to_modify.file_path.clone();
      file_path.set_extension(asset_to_modify.file_type.extension());

      plugins.transformers(&asset_to_modify).await?
    };

    let RunPipelineOutput {
      invalidations,
      discovered_assets,
      pipeline_result: result,
      ..
    } = request_context
      .cache
      .run(
        RunPipelineInput {
          asset: asset_to_modify,
          pipeline,
          plugins: plugins.clone(),
          project_root: request_context.project_root.clone(),
        },
        |input| async { run_pipeline(input).await },
      )
      .await?;

    all_invalidations.extend(invalidations);
    asset_queue.extend(
      discovered_assets
        .into_iter()
        .map(|discovered_asset| (discovered_asset, None)),
    );

    match result {
      PipelineResult::Complete(asset, mut dependencies) => {
        dependencies.extend(prev_dependencies);

        // We assume the first asset to complete the pipeline is the initial asset
        if initial_asset.is_none() {
          initial_asset = Some(asset);
          initial_asset_dependencies = Some(dependencies);
        } else {
          processed_assets.push(AssetWithDependencies {
            asset,
            dependencies,
          });
        };
      }
      PipelineResult::TypeChange(asset, mut dependencies) => {
        let next_pipeline = plugins.transformers(&asset).await?;
        dependencies.extend(prev_dependencies);

        asset_queue.push_front((
          AssetWithDependencies {
            asset: asset.clone(),
            dependencies,
          },
          Some(next_pipeline),
        ));
      }
    }
  }

  Ok(TransformResult {
    asset: initial_asset
      .ok_or_else(|| anyhow!("Initial asset missing after transformer pipeline"))?,
    dependencies: initial_asset_dependencies
      .ok_or_else(|| anyhow!("Initial asset missing after transformer pipeline"))?,
    discovered_assets: processed_assets,
    invalidate_on_file_change: all_invalidations,

    // TODO: Remove this as we've already done caching and this doesn't mean anything at this level
    cache_bailout: false,
    symbol_info: TransformSymbolInfo::default(),
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::plugins::MockPlugins;
  use crate::plugins::TransformerPipeline;
  use crate::request_tracker::RunRequestContext;
  use atlaspack_core::hash::IdentifierHasher;
  use atlaspack_core::plugin::TransformerPlugin;
  use atlaspack_core::types::{Code, FileType};
  use pretty_assertions::assert_eq;
  use std::hash::Hasher;
  use std::sync::Arc;

  fn make_asset(file_path: &str, file_type: FileType) -> Asset {
    Asset {
      file_path: PathBuf::from(file_path),
      file_type,
      ..Default::default()
    }
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
    plugins.expect_transformers().returning(move |_| {
      Ok(TransformerPipeline::new(vec![
        make_transformer(MockTrasformerOptions {
          label: "transformer1",
          ..Default::default()
        }),
        make_transformer(MockTrasformerOptions {
          label: "transformer2",
          ..Default::default()
        }),
      ]))
    });

    let asset = Asset::default();
    let request_context = RunRequestContext::new_for_testing(Arc::new(plugins));
    let result = run_pipelines(asset, request_context).await.unwrap();

    assert_code(&result.asset, "::transformer1::transformer2");
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_run_pipelines_with_invalidations() {
    let mut plugins = MockPlugins::new();
    plugins
      .expect_transformers()
      .withf(|asset: &Asset| asset.file_path.extension().is_some_and(|ext| ext == "js"))
      .returning(move |_| {
        Ok(TransformerPipeline::new(vec![
          make_transformer(MockTrasformerOptions {
            label: "js-1",
            ..Default::default()
          }),
          make_transformer(MockTrasformerOptions {
            label: "js-2",
            invalidate_on_file_change: Some(vec![PathBuf::from("./tmp")]),
            ..Default::default()
          }),
        ]))
      });

    let asset = make_asset("index.js", FileType::Js);
    let expected_invalidations = vec![PathBuf::from("./tmp")];
    let request_context = RunRequestContext::new_for_testing(Arc::new(plugins));
    let result = run_pipelines(asset, request_context).await.unwrap();

    assert_code(&result.asset, "::js-1::js-2");
    assert_eq!(result.invalidate_on_file_change, expected_invalidations);
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_run_pipelines_with_updated_file_type() {
    let mut plugins = MockPlugins::new();
    plugins
      .expect_transformers()
      .withf(|asset: &Asset| asset.file_path.extension().is_some_and(|ext| ext == "json"))
      .returning(move |_| {
        Ok(TransformerPipeline::new(vec![make_transformer(
          MockTrasformerOptions {
            label: "json",
            updated_file_type: Some(FileType::Js),
            ..Default::default()
          },
        )]))
      })
      .times(1);

    plugins
      .expect_transformers()
      .withf(|asset: &Asset| asset.file_type == FileType::Js)
      .returning(move |_| {
        Ok(TransformerPipeline::new(vec![make_transformer(
          MockTrasformerOptions {
            label: "js",
            ..Default::default()
          },
        )]))
      });

    let asset = make_asset("index.json", FileType::Json);
    let request_context = RunRequestContext::new_for_testing(Arc::new(plugins));
    let result = run_pipelines(asset, request_context).await.unwrap();

    assert_eq!(
      result.asset.clone(),
      Asset {
        id: "531e3635c3398c55".into(),
        file_type: FileType::Js,
        ..result.asset
      }
    );
    assert_eq!(result.discovered_assets.len(), 0);
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_run_pipelines_with_dependencies() {
    let mut plugins = MockPlugins::new();
    plugins
      .expect_transformers()
      .withf(|asset: &Asset| asset.file_path.extension().is_some_and(|ext| ext == "js"))
      .returning(move |_| {
        Ok(TransformerPipeline::new(vec![
          make_transformer(MockTrasformerOptions {
            label: "js-1",
            ..Default::default()
          }),
          make_transformer(MockTrasformerOptions {
            label: "js-2",
            dependencies: Some(vec![Dependency::default()]),
            ..Default::default()
          }),
        ]))
      });

    let asset = make_asset("index.js", FileType::Js);
    let expected_dependencies = vec![Dependency::default()];
    let request_context = RunRequestContext::new_for_testing(Arc::new(plugins));
    let result = run_pipelines(asset, request_context).await.unwrap();

    assert_code(&result.asset, "::js-1::js-2");
    assert_eq!(result.dependencies, expected_dependencies);
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_run_pipelines_with_discovered_assets() {
    let mut plugins = MockPlugins::new();

    plugins
      .expect_transformers()
      .withf(|asset: &Asset| asset.file_path.extension().is_some_and(|ext| ext == "js"))
      .returning(move |_| {
        Ok(TransformerPipeline::new(vec![
          make_transformer(MockTrasformerOptions {
            label: "js-1",
            ..Default::default()
          }),
          make_transformer(MockTrasformerOptions {
            label: "js-2",
            discovered_assets: Some(vec![AssetWithDependencies {
              asset: make_asset("discovered.css", FileType::Css),
              dependencies: vec![Dependency::default()],
            }]),
            ..Default::default()
          }),
        ]))
      });

    plugins
      .expect_transformers()
      .withf(|asset: &Asset| asset.file_path.extension().is_some_and(|ext| ext == "css"))
      .returning(move |_| {
        Ok(TransformerPipeline::new(vec![
          make_transformer(MockTrasformerOptions {
            label: "css-1",
            ..Default::default()
          }),
          make_transformer(MockTrasformerOptions {
            label: "css-2",
            dependencies: Some(vec![Dependency::default()]),
            ..Default::default()
          }),
        ]))
      });

    let asset = make_asset("index.js", FileType::Js);
    let request_context = RunRequestContext::new_for_testing(Arc::new(plugins));
    let result = run_pipelines(asset, request_context).await.unwrap();

    assert_code(&result.asset, "::js-1::js-2");
    assert_eq!(result.discovered_assets.len(), 1);
    assert_code(&result.discovered_assets[0].asset, "::css-1::css-2");
    assert_eq!(result.discovered_assets[0].dependencies.len(), 2);
  }

  #[derive(Default)]
  struct MockTrasformerOptions<'a> {
    label: &'a str,
    discovered_assets: Option<Vec<AssetWithDependencies>>,
    dependencies: Option<Vec<Dependency>>,
    invalidate_on_file_change: Option<Vec<PathBuf>>,
    updated_file_type: Option<FileType>,
  }
  // Mock transformer that can simulate the expected test behavior
  #[derive(Debug)]
  struct MockTransformerPlugin {
    id: u64,
    label: String,
    discovered_assets: Option<Vec<AssetWithDependencies>>,
    dependencies: Option<Vec<Dependency>>,
    invalidate_on_file_change: Option<Vec<PathBuf>>,
    updated_file_type: Option<FileType>,
  }

  // We need to implement Hash manually since we have a String field
  impl Hash for MockTransformerPlugin {
    fn hash<H: Hasher>(&self, state: &mut H) {
      self.id.hash(state);
      self.label.hash(state);
    }
  }

  impl MockTransformerPlugin {
    fn new(
      label: String,
      discovered_assets: Option<Vec<AssetWithDependencies>>,
      dependencies: Option<Vec<Dependency>>,
      invalidate_on_file_change: Option<Vec<PathBuf>>,
      updated_file_type: Option<FileType>,
    ) -> Self {
      let mut hasher = IdentifierHasher::new();
      label.hash(&mut hasher);
      let id = hasher.finish();

      Self {
        id,
        label,
        discovered_assets,
        dependencies,
        invalidate_on_file_change,
        updated_file_type,
      }
    }
  }

  #[async_trait]
  impl TransformerPlugin for MockTransformerPlugin {
    fn id(&self) -> u64 {
      self.id
    }

    async fn transform(&self, asset: Asset) -> anyhow::Result<TransformResult> {
      let mut asset = asset.clone();

      // Transform the asset content by appending the label
      asset.code = Code::from(format!(
        "{}::{}",
        String::from_utf8(asset.code.bytes().to_vec()).unwrap(),
        self.label
      ));

      // Update file type if specified
      if let Some(file_type) = self.updated_file_type.clone() {
        asset.file_type = file_type;
      }

      Ok(TransformResult {
        asset,
        discovered_assets: self.discovered_assets.clone().unwrap_or_default(),
        dependencies: self.dependencies.clone().unwrap_or_default(),
        invalidate_on_file_change: self.invalidate_on_file_change.clone().unwrap_or_default(),
        cache_bailout: false,
      })
    }
  }

  fn make_transformer(options: MockTrasformerOptions) -> Arc<MockTransformerPlugin> {
    let MockTrasformerOptions {
      label,
      discovered_assets,
      dependencies,
      invalidate_on_file_change,
      updated_file_type,
    } = options;

    Arc::new(MockTransformerPlugin::new(
      label.to_string(),
      discovered_assets,
      dependencies,
      invalidate_on_file_change,
      updated_file_type,
    ))
  }
}
