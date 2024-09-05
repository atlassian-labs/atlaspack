use anyhow::Error;
use atlaspack_core::plugin::TransformResult;
use atlaspack_core::plugin::{PluginContext, TransformerPlugin};
use atlaspack_core::types::{Asset, BundleBehavior};

#[derive(Debug)]
pub struct AtlaspackRawTransformerPlugin {}

impl AtlaspackRawTransformerPlugin {
  pub fn new(_ctx: &PluginContext) -> Result<Self, Error> {
    Ok(AtlaspackRawTransformerPlugin {})
  }
}

impl TransformerPlugin for AtlaspackRawTransformerPlugin {
  fn transform(&mut self, asset: Asset) -> Result<TransformResult, Error> {
    let mut asset = asset.clone();

    asset.bundle_behavior = BundleBehavior::Isolated;

    Ok(TransformResult {
      asset,
      dependencies: Vec::new(),
      invalidate_on_file_change: Vec::new(),
    })
  }
}

#[cfg(test)]
mod test {
  use std::{path::PathBuf, sync::Arc};

  use atlaspack_core::{
    config_loader::ConfigLoader,
    plugin::{PluginLogger, PluginOptions},
  };
  use atlaspack_filesystem::in_memory_file_system::InMemoryFileSystem;

  use super::*;

  #[test]
  fn returns_raw_asset() {
    let file_system = Arc::new(InMemoryFileSystem::default());
    let mut plugin = AtlaspackRawTransformerPlugin::new(&PluginContext {
      config: Arc::new(ConfigLoader {
        fs: file_system.clone(),
        project_root: PathBuf::default(),
        search_path: PathBuf::default(),
      }),
      file_system,
      logger: PluginLogger::default(),
      options: Arc::new(PluginOptions::default()),
    })
    .expect("Expected raw transformer to initialize");

    let asset = Asset::default();

    assert_ne!(asset.bundle_behavior, BundleBehavior::Isolated);
    assert_eq!(
      plugin.transform(asset).map_err(|e| e.to_string()),
      Ok(TransformResult {
        asset: Asset {
          bundle_behavior: BundleBehavior::Isolated,
          ..Asset::default()
        },
        dependencies: Vec::new(),
        invalidate_on_file_change: Vec::new()
      })
    );
  }
}
