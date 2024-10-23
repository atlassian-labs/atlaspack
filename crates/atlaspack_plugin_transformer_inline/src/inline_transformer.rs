use anyhow::Error;
use atlaspack_core::plugin::PluginContext;
use atlaspack_core::plugin::TransformContext;
use atlaspack_core::plugin::TransformResult;
use atlaspack_core::plugin::TransformerPlugin;
use atlaspack_core::types::Asset;
use atlaspack_core::types::BundleBehavior;

#[derive(Debug)]
pub struct AtlaspackInlineTransformerPlugin {}

impl AtlaspackInlineTransformerPlugin {
  pub fn new(_ctx: &PluginContext) -> Self {
    AtlaspackInlineTransformerPlugin {}
  }
}

impl TransformerPlugin for AtlaspackInlineTransformerPlugin {
  fn transform(&self, _context: TransformContext, asset: Asset) -> Result<TransformResult, Error> {
    let mut asset = asset.clone();

    asset.bundle_behavior = Some(BundleBehavior::Inline);

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
  fn returns_inline_string_asset() {
    let file_system = Arc::new(InMemoryFileSystem::default());
    let plugin = AtlaspackInlineTransformerPlugin::new(&PluginContext {
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
    let context = TransformContext::default();

    assert_ne!(asset.bundle_behavior, Some(BundleBehavior::Inline));
    assert_eq!(
      plugin.transform(context, asset).map_err(|e| e.to_string()),
      Ok(TransformResult {
        asset: Asset {
          bundle_behavior: Some(BundleBehavior::Inline),
          ..Asset::default()
        },
        ..Default::default()
      })
    );
  }
}
