use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use atlaspack_config::AtlaspackConfig;
use atlaspack_config::map::NamedPattern;
use atlaspack_core::diagnostic_error;
use atlaspack_core::plugin::PluginContext;
use atlaspack_core::plugin::ResolverPlugin;
use atlaspack_core::plugin::TransformerPlugin;
use atlaspack_core::types::Asset;
use atlaspack_package_manager::PackageManagerRef;
use atlaspack_plugin_resolver::AtlaspackResolver;
use atlaspack_plugin_rpc::RpcWorkerRef;
use atlaspack_plugin_transformer_css::AtlaspackCssTransformerPlugin;
use atlaspack_plugin_transformer_html::AtlaspackHtmlTransformerPlugin;
use atlaspack_plugin_transformer_image::AtlaspackImageTransformerPlugin;
use atlaspack_plugin_transformer_inline::AtlaspackInlineTransformerPlugin;
use atlaspack_plugin_transformer_inline_string::AtlaspackInlineStringTransformerPlugin;
use atlaspack_plugin_transformer_js::AtlaspackJsTransformerPlugin;
use atlaspack_plugin_transformer_json::AtlaspackJsonTransformerPlugin;
use atlaspack_plugin_transformer_raw::AtlaspackRawTransformerPlugin;
use atlaspack_plugin_transformer_svg::AtlaspackSvgTransformerPlugin;
use atlaspack_plugin_transformer_tokens::AtlaspackTokensTransformerPlugin;
use atlaspack_plugin_transformer_yaml::AtlaspackYamlTransformerPlugin;

use super::Plugins;
use super::TransformerPipeline;
use super::plugin_cache::PluginCache;

/// Loads plugins based on the Atlaspack config
pub struct ConfigPlugins {
  rpc_worker: RpcWorkerRef,

  /// The Atlaspack config that determines what plugins will be loaded
  config: AtlaspackConfig,

  /// Dependencies available to all plugin types
  ctx: PluginContext,

  /// Storage of initialized plugins
  plugin_cache: PluginCache,

  package_manager: PackageManagerRef,
}

impl ConfigPlugins {
  pub fn new(
    rpc_worker: RpcWorkerRef,
    config: AtlaspackConfig,
    ctx: PluginContext,
    package_manager: PackageManagerRef,
  ) -> anyhow::Result<Self> {
    Ok(ConfigPlugins {
      rpc_worker,
      config,
      ctx,
      plugin_cache: Default::default(),
      package_manager,
    })
  }

  fn missing_plugin(&self, path: &Path, phase: &str) -> anyhow::Error {
    diagnostic_error!("No {phase} found for path {}", path.display())
  }

  fn missing_pipeline_plugin(&self, path: &Path, phase: &str, pipeline: &str) -> anyhow::Error {
    diagnostic_error!(
      "No {phase} found for path {} with pipeline {pipeline}",
      path.display(),
    )
  }
}

#[async_trait]
impl Plugins for ConfigPlugins {
  fn named_pipelines(&self) -> Vec<String> {
    self.config.transformers.named_pipelines()
  }

  fn resolvers(&self) -> anyhow::Result<Vec<Arc<dyn ResolverPlugin>>> {
    self.plugin_cache.get_or_init_resolvers(move || {
      let mut resolvers: Vec<Arc<dyn ResolverPlugin>> = Vec::new();

      for resolver in self.config.resolvers.iter() {
        if resolver.package_name == "@atlaspack/resolver-default" {
          resolvers.push(Arc::new(AtlaspackResolver::new(&self.ctx)?));
          continue;
        }

        resolvers.push(self.rpc_worker.create_resolver(&self.ctx, resolver)?);
      }

      Ok(resolvers)
    })
  }

  /// Resolve and load transformer plugins for a given path.
  async fn transformers(&self, asset: &Asset) -> Result<TransformerPipeline, anyhow::Error> {
    let mut transformers: Vec<Arc<dyn TransformerPlugin>> = Vec::new();
    let named_pattern = asset.pipeline.as_ref().map(|pipeline| NamedPattern {
      pipeline,
      use_fallback: false,
    });

    let file_path = &asset.file_path.with_extension(asset.file_type.extension());

    for transformer in self
      .config
      .transformers
      .get(file_path, named_pattern)
      .iter()
    {
      let transformer_name = transformer.package_name.as_str();

      match transformer_name {
        // Currently JS plugins don't work and it's easier to just skip these.
        // We also will probably remove babel from the defaults and support react refresh in Rust
        // before releasing native asset graph
        "@atlaspack/transformer-react-refresh-wrap" => continue,
        "@atlaspack/transformer-posthtml" => continue,
        "@atlaspack/transformer-postcss" => continue,
        _ => {}
      }

      let transformer = self
        .plugin_cache
        .get_or_init_transformer(transformer_name, async || {
          Ok(match transformer_name {
            "@atlaspack/transformer-js" => {
              Arc::new(AtlaspackJsTransformerPlugin::new(&self.ctx)?) as Arc<dyn TransformerPlugin>
            }
            "@atlaspack/transformer-css" => {
              Arc::new(AtlaspackCssTransformerPlugin::new(&self.ctx)?) as Arc<dyn TransformerPlugin>
            }
            "@atlaspack/transformer-inline-string" => {
              Arc::new(AtlaspackInlineStringTransformerPlugin::new(&self.ctx))
                as Arc<dyn TransformerPlugin>
            }
            "@atlaspack/transformer-inline" => {
              Arc::new(AtlaspackInlineTransformerPlugin::new(&self.ctx))
                as Arc<dyn TransformerPlugin>
            }
            "@atlaspack/transformer-image" => {
              Arc::new(AtlaspackImageTransformerPlugin::new(&self.ctx))
                as Arc<dyn TransformerPlugin>
            }
            "@atlaspack/transformer-raw" => {
              Arc::new(AtlaspackRawTransformerPlugin::new(&self.ctx)) as Arc<dyn TransformerPlugin>
            }
            "@atlaspack/transformer-html" => {
              Arc::new(AtlaspackHtmlTransformerPlugin::new(&self.ctx)) as Arc<dyn TransformerPlugin>
            }
            "@atlaspack/transformer-json" => {
              Arc::new(AtlaspackJsonTransformerPlugin::new(&self.ctx)) as Arc<dyn TransformerPlugin>
            }
            "@atlaspack/transformer-yaml" => {
              Arc::new(AtlaspackYamlTransformerPlugin::new(&self.ctx)) as Arc<dyn TransformerPlugin>
            }
            "@atlaspack/transformer-svg" => {
              Arc::new(AtlaspackSvgTransformerPlugin::new(&self.ctx)) as Arc<dyn TransformerPlugin>
            }
            "@atlaspack/transformer-tokens" => {
              Arc::new(AtlaspackTokensTransformerPlugin::new(&self.ctx)?)
                as Arc<dyn TransformerPlugin>
            }
            _ => {
              self
                .rpc_worker
                .create_transformer(&self.ctx, transformer, self.package_manager.clone())
                .await?
            }
          })
        })
        .await?;

      if !transformer.should_skip(asset)? {
        transformers.push(transformer);
      }
    }

    if transformers.is_empty() {
      return match asset.pipeline {
        None => Err(self.missing_plugin(&asset.file_path, "transformers")),
        Some(ref pipeline) => {
          Err(self.missing_pipeline_plugin(&asset.file_path, "transformers", pipeline))
        }
      };
    }

    Ok(TransformerPipeline::new(transformers))
  }
}

#[cfg(test)]
mod tests {
  use crate::test_utils::{config_plugins, make_test_plugin_context};

  use super::*;

  #[test]
  fn returns_resolvers() {
    let resolvers = config_plugins(make_test_plugin_context())
      .resolvers()
      .expect("Not to panic");

    assert_eq!(format!("{:?}", resolvers), "[AtlaspackResolver]")
  }

  #[tokio::test]
  async fn returns_transformers() {
    use atlaspack_core::types::{Code, Environment};
    use std::path::PathBuf;
    use std::sync::Arc;

    let ctx = make_test_plugin_context();
    let asset = Asset::new(
      Code::from("console.log('test');"),
      false,
      Arc::new(Environment::default()),
      PathBuf::from("a.ts"),
      None,
      &PathBuf::default(),
      None,
      false,
    )
    .unwrap();

    let pipeline = config_plugins(ctx)
      .transformers(&asset)
      .await
      .expect("Not to panic");

    assert_eq!(
      format!("{:?}", pipeline),
      format!(
        "{:?}",
        TransformerPipeline::new(vec![Arc::new(
          AtlaspackJsTransformerPlugin::new(&make_test_plugin_context()).unwrap()
        )]),
      )
    );
  }
}
