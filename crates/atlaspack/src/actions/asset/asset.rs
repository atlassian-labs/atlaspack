use std::collections::VecDeque;
use std::hash::Hash;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::anyhow;
use atlaspack_core::plugin::*;
use atlaspack_core::types::*;
use atlaspack_filesystem::FileSystemRef;
use atlaspack_resolver::FileSystem;
use atlaspack_sourcemap::find_sourcemap_url;
use atlaspack_sourcemap::load_sourcemap_url;

use super::super::ActionQueue;
use super::super::Compilation;
use crate::actions::path::PathAction;
use crate::actions::Action;
use crate::plugins::config_plugins::ConfigPlugins;
use crate::plugins::Plugins;
use crate::plugins::PluginsRef;
use crate::request_tracker::Request;
use crate::request_tracker::ResultAndInvalidations;
use crate::request_tracker::RunRequestContext;
use crate::request_tracker::RunRequestError;

/// The AssetRequest runs transformer plugins on discovered Assets.
/// - Decides which transformer pipeline to run from the input Asset type
/// - Runs the pipeline in series, switching pipeline if the Asset type changes
/// - Stores the final Asset source code in the cache, for access in packaging
/// - Finally, returns the complete Asset and it's discovered Dependencies
#[derive(Debug, Hash)]
pub struct AssetAction {
  pub project_root: PathBuf,
  pub code: Option<String>,
  pub env: Arc<Environment>,
  pub file_path: PathBuf,
  pub pipeline: Option<String>,
  pub query: Option<String>,
  pub side_effects: bool,
}

impl Action for AssetAction {
  async fn run(
    self,
    q: ActionQueue,
    Compilation {
      fs,
      plugins,
      asset_graph,
      pending_dependency_links,
      ..
    }: &Compilation,
  ) -> anyhow::Result<()> {
    plugins
      .reporter()
      .report(&ReporterEvent::BuildProgress(BuildProgressEvent::Building(
        AssetBuildEvent {
          file_path: self.file_path.clone(),
        },
      )))
      .await?;

    let TransformResult {
      asset,
      dependencies,
      discovered_assets,
      ..
    } = self.exec(fs.clone(), plugins.clone()).await?;

    let mut asset_graph = asset_graph.write().await;

    for dependency in dependencies {
      // let node_index = asset_graph.add_dependency(dependency.clone());

      q.next(crate::actions::ActionType::Path(PathAction {
        dependency: Arc::new(dependency),
        node_index: Default::default(),
        request_id: self.id(),
      }))?;
    }

    Ok(())
  }
}

impl AssetAction {
  async fn exec(&self, fs: FileSystemRef, plugins: PluginsRef) -> anyhow::Result<TransformResult> {
    let start = Instant::now();

    let code = if let Some(code) = self.code.as_ref() {
      Code::from(code.to_owned())
    } else {
      let code_from_disk = fs.read(&self.file_path)?;
      Code::new(code_from_disk)
    };

    let mut asset = Asset::new(
      code,
      self.env.clone(),
      self.file_path.clone(),
      self.pipeline.clone(),
      &self.project_root,
      self.query.clone(),
      self.side_effects,
    );

    // Load an existing sourcemap if available for valid file types
    if matches!(
      asset.file_type,
      FileType::Css | FileType::Js | FileType::Jsx | FileType::Ts | FileType::Tsx,
    ) {
      let code = asset.code.as_str()?;

      if let Some(url_match) = find_sourcemap_url(code) {
        // Sourcemaps are intentionally skipped if they are invalid
        if let Ok(source_map) =
          load_sourcemap_url(&fs, &self.project_root, &asset.file_path, &url_match.url)
        {
          asset.map = Some(source_map);
          asset.code = Code::from(code.replace(&url_match.code, ""));
        }
      }
    }

    let transform_context = TransformContext::new(self.env.clone());
    let mut result = run_pipelines(transform_context, asset, plugins.clone()).await?;

    result.asset.stats = AssetStats {
      size: result.asset.code.size(),
      time: start.elapsed().as_millis().try_into().unwrap_or(u32::MAX),
    };

    Ok(result)
  }
}

pub async fn run_pipelines(
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
      let is_different_asset_type = transform_result.asset.file_type != original_asset_type;
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
      if is_different_asset_type {
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

#[derive(Debug, Clone, PartialEq)]
pub struct AssetRequestOutput {
  pub asset: Asset,
  pub discovered_assets: Vec<AssetWithDependencies>,
  pub dependencies: Vec<Dependency>,
}
