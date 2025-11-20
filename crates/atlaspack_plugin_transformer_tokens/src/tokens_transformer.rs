use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Error, anyhow};
use async_trait::async_trait;
use atlaspack_atlaskit_tokens::{TokensConfig, TokensPluginOptions, process_tokens_sync};
use atlaspack_core::plugin::{PluginContext, PluginOptions, TransformerPlugin};
use atlaspack_core::plugin::{TransformContext, TransformResult};
use atlaspack_core::types::{Asset, Code, Diagnostic, ErrorKind};
use atlaspack_sourcemap::SourceMap as AtlaspackSourceMap;

use crate::tokens_transformer_config::{PackageJson, TokensTransformerConfig};

#[derive(Debug)]
pub struct AtlaspackTokensTransformerPlugin {
  project_root: PathBuf,
  options: Arc<PluginOptions>,
}

impl AtlaspackTokensTransformerPlugin {
  pub fn new(ctx: &PluginContext) -> Result<Self, Error> {
    Ok(AtlaspackTokensTransformerPlugin {
      project_root: ctx.options.project_root.clone(),
      options: ctx.options.clone(),
    })
  }

  fn load_config(
    &self,
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
    context: TransformContext,
    asset: Asset,
  ) -> Result<TransformResult, Error> {
    // Check feature flag first
    if !self
      .options
      .feature_flags
      .bool_enabled("enableTokensTransformer")
    {
      return Ok(TransformResult {
        asset,
        ..Default::default()
      });
    }

    // Check if code contains '@atlaskit/tokens' before processing
    let code_str = asset.code.as_str()?;
    if !code_str.contains("@atlaskit/tokens") {
      return Ok(TransformResult {
        asset,
        ..Default::default()
      });
    }

    // Load config from package.json
    let config = self.load_config(context.config())?;

    let Some(config) = config else {
      // If no config provided, just return asset unchanged
      return Ok(TransformResult {
        asset,
        ..Default::default()
      });
    };

    // Resolve token data path relative to project root
    let token_data_path = if config.token_data_path.starts_with('/') {
      config.token_data_path.clone()
    } else {
      self
        .project_root
        .join(&config.token_data_path)
        .to_string_lossy()
        .to_string()
    };

    // Build tokens config
    let tokens_config = TokensConfig {
      filename: asset.file_path.to_string_lossy().to_string(),
      project_root: self.project_root.to_string_lossy().to_string(),
      is_source: asset.is_source,
      source_maps: asset.env.source_map.is_some(),
      tokens_options: TokensPluginOptions {
        token_data_path,
        should_use_auto_fallback: config.should_use_auto_fallback.unwrap_or(true),
        should_force_auto_fallback: config.should_force_auto_fallback.unwrap_or(true),
        force_auto_fallback_exemptions: config.force_auto_fallback_exemptions.unwrap_or_default(),
        default_theme: config.default_theme.unwrap_or_else(|| "light".to_string()),
      },
    };

    // Process tokens
    let result = process_tokens_sync(code_str, &tokens_config)
      .with_context(|| format!("Failed to process tokens for {}", asset.file_path.display()))?;

    let mut transformed_asset = asset.clone();
    transformed_asset.code = Code::new(result.code.into_bytes());

    // Handle source maps
    let mut invalidate_on_file_change = vec![];
    if let Ok(token_data_path) =
      std::path::Path::new(&tokens_config.tokens_options.token_data_path).canonicalize()
    {
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
