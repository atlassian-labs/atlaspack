use std::sync::Arc;

use anyhow::Error;
use async_trait::async_trait;
use atlaspack_core::plugin::{PluginContext, TransformerPlugin};
use atlaspack_core::plugin::{TransformContext, TransformResult};
use atlaspack_core::types::{Asset, Code, FileType};

#[derive(Debug)]
pub struct AtlaspackYamlTransformerPlugin {}

impl AtlaspackYamlTransformerPlugin {
  pub fn new(_ctx: &PluginContext) -> Self {
    AtlaspackYamlTransformerPlugin {}
  }
}

#[async_trait]
impl TransformerPlugin for AtlaspackYamlTransformerPlugin {
  async fn transform(
    &self,
    _context: TransformContext,
    asset: Asset,
  ) -> Result<TransformResult, Error> {
    let mut asset = asset.clone();

    let code = serde_yml::from_slice::<serde_yml::Value>(asset.code.bytes())?;
    let code = serde_json::to_string(&code)?;

    asset.code = Arc::new(Code::from(format!("module.exports = {code};")));
    asset.file_type = FileType::Js;

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
  use pretty_assertions::assert_eq;

  use super::*;

  fn create_yaml_plugin() -> AtlaspackYamlTransformerPlugin {
    let file_system = Arc::new(InMemoryFileSystem::default());

    AtlaspackYamlTransformerPlugin::new(&PluginContext {
      config: Arc::new(ConfigLoader {
        fs: file_system.clone(),
        project_root: PathBuf::default(),
        search_path: PathBuf::default(),
      }),
      file_system,
      logger: PluginLogger::default(),
      options: Arc::new(PluginOptions::default()),
    })
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_js_asset_from_yaml() {
    let plugin = create_yaml_plugin();

    let asset = Asset {
      code: Arc::new(Code::from(String::from(
        "
          a: 1
          b:
            c: 2
            d: true
          e:
            - f
            - g
        ",
      ))),
      file_type: FileType::Json,
      ..Asset::default()
    };

    let context = TransformContext::default();
    let transformation = plugin
      .transform(context, asset)
      .await
      .map_err(|e| e.to_string());

    assert_eq!(
      transformation,
      Ok(TransformResult {
        asset: Asset {
          code: Arc::new(Code::from(String::from(
            "module.exports = {\"a\":1,\"b\":{\"c\":2,\"d\":true},\"e\":[\"f\",\"g\"]};"
          ))),
          file_type: FileType::Js,
          ..Asset::default()
        },
        ..Default::default()
      })
    );
  }
}
