use std::path::{Path, PathBuf};

use anyhow::{Context, Error, anyhow};
use async_trait::async_trait;
use atlaspack_atlaskit_tokens::{AtlaskitTokensHandler, TokensConfig, TokensPluginOptions};
use atlaspack_core::plugin::{PluginContext, TransformerPlugin};
use atlaspack_core::plugin::{TransformContext, TransformResult};
use atlaspack_core::types::{Asset, Code, Diagnostic, ErrorKind};
use atlaspack_sourcemap::SourceMap as AtlaspackSourceMap;

use crate::tokens_transformer_config::{PackageJson, TokensTransformerConfig};

#[derive(Debug)]
pub struct AtlaspackTokensTransformerPlugin {
  tokens_handler: Option<AtlaskitTokensHandler>,
  token_data_path: String,
  project_root: PathBuf,
}

impl AtlaspackTokensTransformerPlugin {
  pub fn new(ctx: &PluginContext) -> Result<Self, Error> {
    let project_root = ctx.options.project_root.clone();

    if !ctx
      .options
      .feature_flags
      .bool_enabled("enableTokensTransformer")
    {
      return Ok(AtlaspackTokensTransformerPlugin {
        tokens_handler: None,
        token_data_path: String::new(),
        project_root,
      });
    }

    let config = Self::load_config(ctx.config.clone())?;

    let Some(config) = config else {
      return Ok(AtlaspackTokensTransformerPlugin {
        tokens_handler: None,
        token_data_path: String::new(),
        project_root,
      });
    };

    // Resolve token data path relative to project root
    let token_data_path = if config.token_data_path.starts_with('/') {
      config.token_data_path.clone()
    } else {
      project_root
        .join(&config.token_data_path)
        .to_string_lossy()
        .to_string()
    };

    let tokens_handler = Some(AtlaskitTokensHandler::new(
      project_root.to_string_lossy().to_string(),
      TokensPluginOptions {
        token_data_path: token_data_path.clone(),
        should_use_auto_fallback: config.should_use_auto_fallback.unwrap_or(true),
        should_force_auto_fallback: config.should_force_auto_fallback.unwrap_or(true),
        force_auto_fallback_exemptions: config
          .force_auto_fallback_exemptions
          .as_ref()
          .cloned()
          .unwrap_or_default(),
        default_theme: config
          .default_theme
          .as_deref()
          .unwrap_or("light")
          .to_string(),
      },
    )?);

    Ok(AtlaspackTokensTransformerPlugin {
      tokens_handler,
      token_data_path,
      project_root,
    })
  }

  fn load_config(
    config_loader: atlaspack_core::config_loader::ConfigLoaderRef,
  ) -> Result<Option<TokensTransformerConfig>, Error> {
    let config = config_loader
      .load_package_json::<PackageJson>()
      .map_or_else(
        |err| {
          let diagnostic = err.downcast_ref::<Diagnostic>();

          if diagnostic.is_some_and(|d| d.kind != ErrorKind::NotFound) {
            return Err(err);
          }

          Ok(None)
        },
        |config| Ok(config.contents.config),
      )?;

    Ok(config)
  }
}

#[async_trait]
impl TransformerPlugin for AtlaspackTokensTransformerPlugin {
  async fn transform(
    &self,
    _ctx: TransformContext,
    asset: Asset,
  ) -> Result<TransformResult, Error> {
    let Some(tokens_handler) = &self.tokens_handler else {
      return Ok(TransformResult {
        asset,
        ..Default::default()
      });
    };

    // Check if code contains '@atlaskit/tokens' before processing
    let code_str = asset.code.as_str()?;
    if !code_str.contains("@atlaskit/tokens") {
      return Ok(TransformResult {
        asset,
        ..Default::default()
      });
    }

    // Build tokens config
    let tokens_config = TokensConfig {
      filename: asset.file_path.to_string_lossy().to_string(),
      is_source: asset.is_source,
      source_maps: asset.env.source_map.is_some(),
    };

    // Process tokens
    let result = tokens_handler
      .process(code_str, tokens_config)
      .with_context(|| format!("Failed to process tokens for {}", asset.file_path.display()))?;

    let mut transformed_asset = asset.clone();
    transformed_asset.code = Code::new(result.code.into_bytes());

    // Handle source maps
    let mut invalidate_on_file_change = vec![];
    if let Ok(token_data_path) = Path::new(&self.token_data_path).canonicalize() {
      invalidate_on_file_change.push(token_data_path);
    }

    if let Some(map_json) = result.map {
      let mut source_map = AtlaspackSourceMap::from_json(&self.project_root, &map_json)
        .map_err(|e| anyhow!("Failed to parse new source map: {}", e))?;

      if let Some(existing_map) = &transformed_asset.map {
        source_map
          .extends(&mut existing_map.clone())
          .map_err(|e| anyhow!("Failed to extend source map: {}", e))?;
      }

      transformed_asset.map = Some(source_map);
    }

    Ok(TransformResult {
      asset: transformed_asset,
      invalidate_on_file_change,
      ..Default::default()
    })
  }
}
