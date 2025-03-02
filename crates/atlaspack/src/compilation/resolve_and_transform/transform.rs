use std::collections::VecDeque;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use atlaspack_core::config_loader::ConfigLoader;
use atlaspack_core::plugin::TransformContext;
use atlaspack_core::plugin::TransformResult;
use atlaspack_core::types::Asset;
use atlaspack_core::types::AssetStats;
use atlaspack_core::types::AssetWithDependencies;
use atlaspack_core::types::Code;
use atlaspack_core::types::Dependency;
use atlaspack_core::types::Environment;
use atlaspack_core::types::FileType;
use atlaspack_filesystem::FileSystemRef;
use atlaspack_sourcemap::find_sourcemap_url;
use atlaspack_sourcemap::load_sourcemap_url;

use crate::plugins::PluginsRef;

#[derive(Debug, Clone, PartialEq)]
pub struct TransformationResult {
  pub asset: Asset,
  pub discovered_assets: Vec<AssetWithDependencies>,
  pub dependencies: Vec<Dependency>,
}

pub async fn transform(
  fs: FileSystemRef,
  project_root: &PathBuf,
  plugins: PluginsRef,
  code: Option<String>,
  env: Arc<Environment>,
  file_path: PathBuf,
  pipeline: Option<String>,
  query: Option<String>,
  side_effects: bool,
) -> anyhow::Result<TransformationResult> {
  let start = Instant::now();

  let is_virtual = code.is_some();

  let code = if let Some(code) = code.as_ref() {
    Code::from(code.to_owned())
  } else {
    let code_from_disk = fs.read(&file_path)?;
    Code::new(code_from_disk)
  };

  let mut asset = Asset::new(
    code,
    is_virtual,
    env.clone(),
    file_path.clone(),
    pipeline.clone(),
    &project_root,
    query.clone(),
    side_effects,
  )?;

  // Load an existing sourcemap if available for valid file types
  if matches!(
    asset.file_type,
    FileType::Css | FileType::Js | FileType::Jsx | FileType::Ts | FileType::Tsx,
  ) {
    let code = asset.code.as_str()?;

    if let Some(url_match) = find_sourcemap_url(code) {
      // Sourcemaps are intentionally skipped if they are invalid
      if let Ok(source_map) =
        load_sourcemap_url(&fs, &project_root, &asset.file_path, &url_match.url)
      {
        asset.map = Some(source_map);
        asset.code = Code::from(code.replace(&url_match.code, ""));
      }
    }
  }

  let config_loader = Arc::new(ConfigLoader {
    fs: fs.clone(),
    project_root: project_root.clone(),
    search_path: asset.file_path.clone(),
  });

  let transform_context = TransformContext::new(config_loader, asset.env.clone());

  let mut result = run_pipelines(transform_context, asset, plugins.clone(), &project_root).await?;

  // TODO: Commit the asset with a project path now, as transformers rely on an absolute path
  // result.asset.file_path = to_project_path(&self.project_root, &result.asset.file_path);

  result.asset.stats = AssetStats {
    size: result.asset.code.size(),
    time: start.elapsed().as_millis().try_into().unwrap_or(u32::MAX),
  };

  Ok(TransformationResult {
    asset: result.asset,
    // TODO: Need to decide whether a discovered asset will belong to the asset graph as it's own node
    discovered_assets: result.discovered_assets,
    dependencies: result.dependencies,
  })
}

async fn run_pipelines(
  transform_context: TransformContext,
  input: Asset,
  plugins: PluginsRef,
  project_root: &Path,
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
    let mut pipeline_complete = true;

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
        // When the Asset changes file_type we need to regenerate its id
        current_asset.update_id(project_root);

        let next_pipeline = plugins.transformers(
          &current_asset
            .file_path
            .with_extension(current_asset.file_type.extension()),
          current_asset.pipeline.clone(),
        )?;

        let next_pipeline_id = next_pipeline.id();

        if next_pipeline_id != pipeline_id {
          asset_queue.push_front((
            AssetWithDependencies {
              asset: current_asset.clone(),
              dependencies: current_dependencies.clone(),
            },
            Some((next_pipeline, next_pipeline_id)),
          ));
          pipeline_complete = false;
          break;
        }
      }
    }

    if pipeline_complete {
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
  }

  Ok(TransformResult {
    asset: initial_asset
      .ok_or_else(|| anyhow::anyhow!("Initial asset missing after transformer pipeline"))?,
    dependencies: initial_asset_dependencies
      .ok_or_else(|| anyhow::anyhow!("Initial asset missing after transformer pipeline"))?,
    discovered_assets: processed_assets,
    invalidate_on_file_change: invalidations,
  })
}
