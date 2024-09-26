use anyhow::Error;
use atlaspack_core::plugin::{PluginContext, TransformResult, TransformerPlugin};
use atlaspack_core::types::{Asset, BundleBehavior};

#[derive(Debug)]
pub struct AtlaspackInlineStringTransformerPlugin {}

impl AtlaspackInlineStringTransformerPlugin {
  pub fn new(_ctx: &PluginContext) -> Self {
    AtlaspackInlineStringTransformerPlugin {}
  }
}

impl TransformerPlugin for AtlaspackInlineStringTransformerPlugin {
  fn transform(&mut self, asset: Asset) -> Result<TransformResult, Error> {
    let mut asset = asset.clone();

    asset.bundle_behavior = BundleBehavior::Inline;
    asset
      .meta
      .insert(String::from("inlineType"), "string".into());

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
    types::JSONObject,
  };
  use atlaspack_filesystem::in_memory_file_system::InMemoryFileSystem;

  use super::*;

  #[test]
  fn returns_inline_string_asset() {
    let file_system = Arc::new(InMemoryFileSystem::default());
    let mut plugin = AtlaspackInlineStringTransformerPlugin::new(&PluginContext {
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

    assert_ne!(asset.bundle_behavior, BundleBehavior::Inline);
    assert_eq!(
      plugin.transform(asset).map_err(|e| e.to_string()),
      Ok(TransformResult {
        asset: Asset {
          bundle_behavior: BundleBehavior::Inline,
          meta: JSONObject::from_iter([(String::from("inlineType"), "string".into())]),
          ..Asset::default()
        },
        ..Default::default()
      })
    );
  }
}
