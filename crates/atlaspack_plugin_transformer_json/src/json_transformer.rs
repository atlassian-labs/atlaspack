use std::sync::Arc;

use anyhow::Error;
use atlaspack_core::plugin::TransformResult;
use atlaspack_core::plugin::{PluginContext, TransformerPlugin};
use atlaspack_core::types::{Asset, Code, FileType};

#[derive(Debug)]
pub struct AtlaspackJsonTransformerPlugin {}

impl AtlaspackJsonTransformerPlugin {
  pub fn new(_ctx: &PluginContext) -> Result<Self, Error> {
    Ok(AtlaspackJsonTransformerPlugin {})
  }
}

impl TransformerPlugin for AtlaspackJsonTransformerPlugin {
  fn transform(&mut self, asset: Asset) -> Result<TransformResult, Error> {
    let mut asset = asset.clone();

    let code = std::str::from_utf8(asset.code.bytes())?;
    let code = json5::from_str::<serde_json::Value>(code)?;
    let code = json5::to_string(&code)?;
    let code = json::stringify(code);

    asset.code = Arc::new(Code::from(format!("module.exports = JSON.parse({code});")));
    asset.file_type = FileType::Js;

    Ok(TransformResult {
      asset,
      dependencies: Vec::new(),
      invalidate_on_file_change: Vec::new(),
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

  fn create_json_plugin() -> AtlaspackJsonTransformerPlugin {
    let file_system = Arc::new(InMemoryFileSystem::default());

    AtlaspackJsonTransformerPlugin::new(&PluginContext {
      config: Arc::new(ConfigLoader {
        fs: file_system.clone(),
        project_root: PathBuf::default(),
        search_path: PathBuf::default(),
      }),
      file_system,
      logger: PluginLogger::default(),
      options: Arc::new(PluginOptions::default()),
    })
    .expect("Expected json transformer to initialize")
  }

  #[test]
  fn returns_js_asset_from_json() {
    let mut plugin = create_json_plugin();

    let asset = Asset {
      code: Arc::new(Code::from(
        r#"
          {
            "a": "b",
            "c": {
              "d": true,
              "e": 1
            }
          }
        "#
        .to_string(),
      )),
      file_type: FileType::Json,
      ..Asset::default()
    };

    assert_eq!(
      plugin.transform(asset).map_err(|e| e.to_string()),
      Ok(TransformResult {
        asset: Asset {
          code: Arc::new(Code::from(
            r#"module.exports = JSON.parse("{\"a\":\"b\",\"c\":{\"d\":true,\"e\":1}}");"#
              .to_string()
          )),
          file_type: FileType::Js,
          ..Asset::default()
        },
        dependencies: Vec::new(),
        invalidate_on_file_change: Vec::new()
      })
    );
  }

  #[test]
  fn returns_js_asset_from_json5() {
    let mut plugin = create_json_plugin();

    let asset = Asset {
      code: Arc::new(Code::from(
        r#"
          /* start */
          {
            // leading
            "a": "b",
            "c": {
              "d": true, // inline
              "e": 1,
            },
            /* trailing */
          }
          /* end */
        "#
        .to_string(),
      )),
      file_type: FileType::Json,
      ..Asset::default()
    };

    assert_eq!(
      plugin.transform(asset).map_err(|e| e.to_string()),
      Ok(TransformResult {
        asset: Asset {
          code: Arc::new(Code::from(
            r#"module.exports = JSON.parse("{\"a\":\"b\",\"c\":{\"d\":true,\"e\":1}}");"#
              .to_string()
          )),
          file_type: FileType::Js,
          ..Asset::default()
        },
        dependencies: Vec::new(),
        invalidate_on_file_change: Vec::new()
      })
    );
  }
}