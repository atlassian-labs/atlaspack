use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Error};
use atlaspack_core::plugin::{PluginContext, TransformerPlugin};
use atlaspack_core::plugin::{TransformContext, TransformResult};
use atlaspack_core::types::engines::Engines;
use atlaspack_core::types::{Asset, BundleBehavior, Code, EnvironmentContext};
use lightningcss::dependencies::DependencyOptions;
use lightningcss::printer::PrinterOptions;
use lightningcss::stylesheet::{self, ParserFlags, ParserOptions, StyleSheet};
use lightningcss::targets::Targets;

#[derive(Debug)]
pub struct AtlaspackCssTransformerPlugin {
  project_root: PathBuf,
}

impl AtlaspackCssTransformerPlugin {
  pub fn new(ctx: &PluginContext) -> Self {
    AtlaspackCssTransformerPlugin {
      project_root: ctx.options.project_root.clone(),
    }
  }
}

impl TransformerPlugin for AtlaspackCssTransformerPlugin {
  fn transform(
    &mut self,
    context: TransformContext,
    asset: Asset,
  ) -> Result<TransformResult, Error> {
    let result = StyleSheet::parse(
      asset.code.as_str()?,
      ParserOptions {
        filename: asset
          .file_path
          .clone()
          .into_os_string()
          .into_string()
          .map_err(|_e| anyhow!("Couldn't convert file path to String"))?,
        css_modules: None,
        source_index: Default::default(),
        error_recovery: false,
        warnings: None,
        flags: ParserFlags::empty(),
      },
    );

    match result {
      Err(_err) => {
        // TODO: Workout how to handle the code lifetime for errors
        Err(anyhow!("failure"))
      }
      Ok(stylesheet) => {
        let mut asset = asset.clone();

        //  Normalize the asset's environment so that properties that only affect JS don't cause CSS to be duplicated.
        // For example, with ESModule and CommonJS targets, only a single shared CSS bundle should be produced.
        asset.env = Arc::new(atlaspack_core::types::Environment {
          context: EnvironmentContext::Browser,
          engines: Engines {
            browsers: asset.env.engines.browsers.clone(),
            ..Default::default()
          },
          ..asset.env.deref().clone()
        });

        let preserve_imports = asset
          .meta
          .get("hasDependencies")
          .map_or_else(|| true, |value| value != false);

        let css = stylesheet.to_css(PrinterOptions {
          minify: false,
          source_map: None,
          project_root: self.project_root.to_str(),
          targets: Targets {
            // TODO: provide this correctly
            browsers: None,
            include: Default::default(),
            exclude: Default::default(),
          },
          analyze_dependencies: Some(DependencyOptions {
            remove_imports: !preserve_imports,
          }),
          pseudo_classes: None,
        })?;

        asset.code = Arc::new(Code::from(css.code));

        Ok(TransformResult {
          asset,
          ..Default::default()
        })
      }
    }
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
    let mut plugin = AtlaspackCssTransformerPlugin::new(&PluginContext {
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
