use anyhow::Error;
use atlaspack_core::plugin::{PluginContext, TransformerPlugin};
use atlaspack_core::plugin::{TransformContext, TransformResult};
use atlaspack_core::types::{Asset, BundleBehavior};

#[derive(Debug)]
pub struct AtlaspackRawTransformerPlugin {}

impl AtlaspackRawTransformerPlugin {
  pub fn new(_ctx: &PluginContext) -> Self {
    AtlaspackRawTransformerPlugin {}
  }
}

impl TransformerPlugin for AtlaspackRawTransformerPlugin {
  fn transform(&self, _context: TransformContext, asset: Asset) -> Result<TransformResult, Error> {
    let mut asset = asset.clone();

    asset.bundle_behavior = Some(BundleBehavior::Isolated);

    Ok(TransformResult {
      asset,
      ..Default::default()
    })
  }
}

#[cfg(test)]
mod tests {
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
    let plugin = AtlaspackRawTransformerPlugin::new(&PluginContext {
      config: Arc::new(ConfigLoader {
        fs: file_system.clone(),
        project_root: PathBuf::default(),
        search_path: PathBuf::default(),
      }),
      file_system,
      logger: PluginLogger::default(),
      options: Arc::new(PluginOptions::default()),
    });

    let asset = Asset::default();

    assert_ne!(asset.bundle_behavior, Some(BundleBehavior::Isolated));
    let mut asset = asset;
    asset.bundle_behavior = Some(BundleBehavior::Isolated);
    let context = TransformContext::default();

    assert_eq!(
      plugin
        .transform(context, asset.clone())
        .map_err(|e| e.to_string()),
      Ok(TransformResult {
        asset,
        ..Default::default()
      })
    );
  }
}
