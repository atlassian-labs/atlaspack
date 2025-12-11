use anyhow::Error;
use async_trait::async_trait;
use atlaspack_core::plugin::TransformResult;
use atlaspack_core::plugin::{PluginContext, TransformerPlugin};
use atlaspack_core::types::{Asset, Code, FileType};

/// Escape JSON string for embedding in JavaScript double-quoted string
fn escape_for_double_quotes(input: &str) -> String {
  input
    .replace('\\', "\\\\") // Escape backslashes first
    .replace('"', "\\\"") // Escape double quotes
}

#[derive(Debug, Hash)]
pub struct AtlaspackJsonTransformerPlugin {}

impl AtlaspackJsonTransformerPlugin {
  pub fn new(_ctx: &PluginContext) -> Self {
    AtlaspackJsonTransformerPlugin {}
  }
}

#[async_trait]
impl TransformerPlugin for AtlaspackJsonTransformerPlugin {
  #[tracing::instrument(
    level = "debug",
    skip_all,
    fields(plugin = "AtlaspackJsonTransformerPlugin")
  )]
  async fn transform(&self, asset: Asset) -> Result<TransformResult, Error> {
    // First attempt: Try to parse with serde_json as it's much faster than
    // json5 deserialization and 99% of JSON files are standard JSON.
    let js_code = match serde_json::from_slice::<serde_json::Value>(asset.code.bytes()) {
      Ok(parsed) => {
        let json_string = serde_json::to_string(&parsed)?;
        format!(
          "module.exports = JSON.parse(\"{}\");",
          escape_for_double_quotes(&json_string)
        )
      }
      Err(_serde_error) => {
        // Fallback: json5 for JSON5 features (comments, trailing commas, etc.)
        tracing::debug!(
          "Falling back to json5 parser for {} (likely contains JSON5 features)",
          asset.file_path.display()
        );
        let code = std::str::from_utf8(asset.code.bytes())?;
        let parsed = json5::from_str::<serde_json::Value>(code)?;
        let json_string = serde_json::to_string(&parsed)?;
        format!(
          "module.exports = JSON.parse(\"{}\");",
          escape_for_double_quotes(&json_string)
        )
      }
    };

    let code = Code::from(js_code);

    Ok(TransformResult {
      asset: Asset {
        code,
        file_type: FileType::Js,
        ..asset
      },
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
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_js_asset_from_json_serde_path() {
    let plugin = create_json_plugin();

    let asset = Asset {
      code: Code::from(
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
      ),
      file_type: FileType::Json,
      ..Asset::default()
    };

    let result = plugin.transform(asset).await.unwrap();

    // Uses serde_json, which parses and minifies the JSON
    assert_eq!(
      std::str::from_utf8(result.asset.code.bytes()).unwrap(),
      r#"module.exports = JSON.parse("{\"a\":\"b\",\"c\":{\"d\":true,\"e\":1}}");"#
    );
    assert_eq!(result.asset.file_type, FileType::Js);
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_js_asset_from_json_with_whitespace() {
    let plugin = create_json_plugin();

    // JSON with extra whitespace - serde_json will parse and minify
    let asset = Asset {
      code: Code::from(r#"   {"compact":true,"no_spaces":false,"value":123}   "#.to_string()),
      file_type: FileType::Json,
      ..Asset::default()
    };

    let result = plugin.transform(asset).await.unwrap();

    // Uses serde_json, which parses and minifies the JSON
    assert_eq!(
      std::str::from_utf8(result.asset.code.bytes()).unwrap(),
      r#"module.exports = JSON.parse("{\"compact\":true,\"no_spaces\":false,\"value\":123}");"#
    );
    assert_eq!(result.asset.file_type, FileType::Js);
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn tests_serde_json_medium_path() {
    let plugin = create_json_plugin();

    // Create JSON that will bypass fast path (contains slash) but is valid for serde_json
    let asset = Asset {
      code: Code::from(r#"not_an_object"#.to_string()),
      file_type: FileType::Json,
      ..Asset::default()
    };

    // This should fail fast path, then use serde_json, which should also fail since it's invalid JSON
    let result = plugin.transform(asset).await;
    assert!(result.is_err()); // Should error because "not_an_object" is not valid JSON
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn tests_json5_fallback_path() {
    let plugin = create_json_plugin();

    // Create JSON with comments that will fail fast path and serde_json but succeed with json5
    let asset = Asset {
      code: Code::from(
        r#"{
          // This is a comment
          "key": "value"
        }"#
          .to_string(),
      ),
      file_type: FileType::Json,
      ..Asset::default()
    };

    let result = plugin.transform(asset).await.unwrap();

    // Should fall back to json5, parse successfully, then minify
    assert_eq!(
      std::str::from_utf8(result.asset.code.bytes()).unwrap(),
      r#"module.exports = JSON.parse("{\"key\":\"value\"}");"#
    );
    assert_eq!(result.asset.file_type, FileType::Js);
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_js_asset_from_json5() {
    let plugin = create_json_plugin();

    let asset = Asset {
      code: Code::from(
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
      ),
      file_type: FileType::Json,
      ..Asset::default()
    };

    assert_eq!(
      plugin.transform(asset).await.map_err(|e| e.to_string()),
      Ok(TransformResult {
        asset: Asset {
          code: Code::from(
            r#"module.exports = JSON.parse("{\"a\":\"b\",\"c\":{\"d\":true,\"e\":1}}");"#
              .to_string()
          ),
          file_type: FileType::Js,
          ..Asset::default()
        },
        ..Default::default()
      })
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_js_asset_from_invalid_json_falls_back_to_json5() {
    let plugin = create_json_plugin();

    // This will fail serde_json due to trailing comma, then succeed with json5
    let asset = Asset {
      code: Code::from(
        r#"{
          "a": "b",
          "c": {
            "d": true,
            "e": 1,
          },
        }"#
          .to_string(),
      ),
      file_type: FileType::Json,
      ..Asset::default()
    };

    let result = plugin.transform(asset).await.unwrap();

    // Falls back to json5 due to trailing comma, then gets minified
    assert_eq!(
      std::str::from_utf8(result.asset.code.bytes()).unwrap(),
      r#"module.exports = JSON.parse("{\"a\":\"b\",\"c\":{\"d\":true,\"e\":1}}");"#
    );
    assert_eq!(result.asset.file_type, FileType::Js);
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn handles_special_characters() {
    let plugin = create_json_plugin();

    // JSON with special characters
    let asset = Asset {
      code: Code::from(
        r#"{"template": "`${variable}`", "backslash": "C:\\path\\file"}"#.to_string(),
      ),
      file_type: FileType::Json,
      ..Asset::default()
    };

    let result = plugin.transform(asset).await.unwrap();

    // serde_json handles the JSON, output is properly escaped for double quotes
    assert_eq!(
      std::str::from_utf8(result.asset.code.bytes()).unwrap(),
      r#"module.exports = JSON.parse("{\"template\":\"`${variable}`\",\"backslash\":\"C:\\\\path\\\\file\"}");"#
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn handles_arrays() {
    let plugin = create_json_plugin();

    let asset = Asset {
      code: Code::from(r#"["item1", "item2", {"nested": true}]"#.to_string()),
      file_type: FileType::Json,
      ..Asset::default()
    };

    let result = plugin.transform(asset).await.unwrap();

    // Uses serde_json for arrays
    assert_eq!(
      std::str::from_utf8(result.asset.code.bytes()).unwrap(),
      r#"module.exports = JSON.parse("[\"item1\",\"item2\",{\"nested\":true}]");"#
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn handles_urls_with_slashes() {
    let plugin = create_json_plugin();

    // JSON with URL that contains "//" - serde_json handles it fine
    let asset = Asset {
      code: Code::from(r#"{"url": "https://example.com", "valid": true}"#.to_string()),
      file_type: FileType::Json,
      ..Asset::default()
    };

    let result = plugin.transform(asset).await.unwrap();

    // serde_json handles URLs properly
    assert_eq!(
      std::str::from_utf8(result.asset.code.bytes()).unwrap(),
      r#"module.exports = JSON.parse("{\"url\":\"https://example.com\",\"valid\":true}");"#
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn detects_actual_comments() {
    let plugin = create_json_plugin();

    // JSON with actual comments - serde_json will fail, json5 will handle
    let asset = Asset {
      code: Code::from(
        r#"{ // This is a comment
          "key": "value"
        }"#
          .to_string(),
      ),
      file_type: FileType::Json,
      ..Asset::default()
    };

    let result = plugin.transform(asset).await.unwrap();

    // Falls back to json5 due to comments, then gets minified
    assert_eq!(
      std::str::from_utf8(result.asset.code.bytes()).unwrap(),
      r#"module.exports = JSON.parse("{\"key\":\"value\"}");"#
    );
  }
}
