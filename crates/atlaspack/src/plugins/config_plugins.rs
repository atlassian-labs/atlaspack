use std::path::Path;
use std::sync::Arc;

use atlaspack_config::map::NamedPattern;
use atlaspack_config::AtlaspackConfig;
use atlaspack_core::diagnostic_error;
use atlaspack_core::plugin::composite_reporter_plugin::CompositeReporterPlugin;
use atlaspack_core::plugin::BundlerPlugin;
use atlaspack_core::plugin::CompressorPlugin;
use atlaspack_core::plugin::NamerPlugin;
use atlaspack_core::plugin::OptimizerPlugin;
use atlaspack_core::plugin::PackagerPlugin;
use atlaspack_core::plugin::PluginContext;
use atlaspack_core::plugin::ReporterPlugin;
use atlaspack_core::plugin::ResolverPlugin;
use atlaspack_core::plugin::RuntimePlugin;
use atlaspack_core::plugin::TransformerPlugin;
use atlaspack_core::plugin::ValidatorPlugin;
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
use atlaspack_plugin_transformer_yaml::AtlaspackYamlTransformerPlugin;

use super::plugin_cache::PluginCache;
use super::Plugins;
use super::TransformerPipeline;

/// Loads plugins based on the Atlaspack config
pub struct ConfigPlugins {
  rpc_worker: RpcWorkerRef,

  /// The Atlaspack config that determines what plugins will be loaded
  config: AtlaspackConfig,

  /// Dependencies available to all plugin types
  ctx: PluginContext,

  /// A reporter that runs all reporter plugins
  reporter: Arc<dyn ReporterPlugin>,

  /// Storage of initialized plugins
  plugin_cache: PluginCache,
}

impl ConfigPlugins {
  pub fn new(
    rpc_worker: RpcWorkerRef,
    config: AtlaspackConfig,
    ctx: PluginContext,
  ) -> anyhow::Result<Self> {
    let mut reporters: Vec<Box<dyn ReporterPlugin>> = Vec::new();

    for reporter in config.reporters.iter() {
      reporters.push(rpc_worker.create_reporter(&ctx, reporter)?);
    }

    let reporter = Arc::new(CompositeReporterPlugin::new(reporters));

    Ok(ConfigPlugins {
      rpc_worker,
      config,
      ctx,
      reporter,
      plugin_cache: Default::default(),
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

impl Plugins for ConfigPlugins {
  #[allow(unused)]
  fn bundler(&self) -> Result<Box<dyn BundlerPlugin>, anyhow::Error> {
    self
      .rpc_worker
      .create_bundler(&self.ctx, &self.config.bundler)
  }

  #[allow(unused)]
  fn compressors(&self, path: &Path) -> Result<Vec<Box<dyn CompressorPlugin>>, anyhow::Error> {
    let mut compressors: Vec<Box<dyn CompressorPlugin>> = Vec::new();

    for compressor in self.config.compressors.get(path).iter() {
      compressors.push(self.rpc_worker.create_compressor(&self.ctx, compressor)?);
    }

    if compressors.is_empty() {
      return Err(self.missing_plugin(path, "compressors"));
    }

    Ok(compressors)
  }

  fn named_pipelines(&self) -> Vec<String> {
    self.config.transformers.named_pipelines()
  }

  #[allow(unused)]
  fn namers(&self) -> Result<Vec<Box<dyn NamerPlugin>>, anyhow::Error> {
    let mut namers: Vec<Box<dyn NamerPlugin>> = Vec::new();

    for namer in self.config.namers.iter() {
      namers.push(self.rpc_worker.create_namer(&self.ctx, namer)?);
    }

    Ok(namers)
  }

  #[allow(unused)]
  fn optimizers(
    &self,
    path: &Path,
    pipeline: Option<String>,
  ) -> Result<Vec<Box<dyn OptimizerPlugin>>, anyhow::Error> {
    let mut optimizers: Vec<Box<dyn OptimizerPlugin>> = Vec::new();
    let named_pattern = pipeline.as_ref().map(|pipeline| NamedPattern {
      pipeline,
      use_fallback: true,
    });

    for optimizer in self.config.optimizers.get(path, named_pattern).iter() {
      optimizers.push(self.rpc_worker.create_optimizer(&self.ctx, optimizer)?);
    }

    Ok(optimizers)
  }

  #[allow(unused)]
  fn packager(&self, path: &Path) -> Result<Box<dyn PackagerPlugin>, anyhow::Error> {
    let packager = self.config.packagers.get(path);

    let Some(packager) = packager else {
      return Err(self.missing_plugin(path, "packager"));
    };

    self.rpc_worker.create_packager(&self.ctx, packager)
  }

  fn reporter(&self) -> Arc<dyn ReporterPlugin> {
    self.reporter.clone()
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

  #[allow(unused)]
  fn runtimes(&self) -> Result<Vec<Box<dyn RuntimePlugin>>, anyhow::Error> {
    let mut runtimes: Vec<Box<dyn RuntimePlugin>> = Vec::new();

    for runtime in self.config.runtimes.iter() {
      runtimes.push(self.rpc_worker.create_runtime(&self.ctx, runtime)?);
    }

    Ok(runtimes)
  }

  /// Resolve and load transformer plugins for a given path.
  fn transformers(
    &self,
    path: &Path,
    pipeline: Option<String>,
  ) -> Result<TransformerPipeline, anyhow::Error> {
    let mut transformers: Vec<Arc<dyn TransformerPlugin>> = Vec::new();
    let named_pattern = pipeline.as_ref().map(|pipeline| NamedPattern {
      pipeline,
      use_fallback: false,
    });

    for transformer in self.config.transformers.get(path, named_pattern).iter() {
      let transformer_name = transformer.package_name.as_str();

      match transformer_name {
        // Currently JS plugins don't work and it's easier to just skip these.
        // We also will probably remove babel from the defaults and support react refresh in Rust
        // before releasing native asset graph
        "@atlaspack/transformer-babel" => continue,
        "@atlaspack/transformer-react-refresh-wrap" => continue,
        "@atlaspack/transformer-posthtml" => continue,
        "@atlaspack/transformer-postcss" => continue,
        _ => {}
      }

      let transformer = self
        .plugin_cache
        .get_or_init_transformer(transformer_name, || {
          Ok(match transformer_name {
            "@atlaspack/transformer-js" => Arc::new(AtlaspackJsTransformerPlugin::new(&self.ctx)?),
            "@atlaspack/transformer-css" => {
              Arc::new(AtlaspackCssTransformerPlugin::new(&self.ctx)?)
            }
            "@atlaspack/transformer-inline-string" => {
              Arc::new(AtlaspackInlineStringTransformerPlugin::new(&self.ctx))
            }
            "@atlaspack/transformer-inline" => {
              Arc::new(AtlaspackInlineTransformerPlugin::new(&self.ctx))
            }
            "@atlaspack/transformer-image" => {
              Arc::new(AtlaspackImageTransformerPlugin::new(&self.ctx))
            }
            "@atlaspack/transformer-raw" => Arc::new(AtlaspackRawTransformerPlugin::new(&self.ctx)),
            "@atlaspack/transformer-html" => {
              Arc::new(AtlaspackHtmlTransformerPlugin::new(&self.ctx))
            }
            "@atlaspack/transformer-json" => {
              Arc::new(AtlaspackJsonTransformerPlugin::new(&self.ctx))
            }
            "@atlaspack/transformer-yaml" => {
              Arc::new(AtlaspackYamlTransformerPlugin::new(&self.ctx))
            }
            _ => self.rpc_worker.create_transformer(&self.ctx, transformer)?,
          })
        })?;

      transformers.push(transformer);
    }

    if transformers.is_empty() {
      return match pipeline {
        None => Err(self.missing_plugin(path, "transformers")),
        Some(pipeline) => Err(self.missing_pipeline_plugin(path, "transformers", &pipeline)),
      };
    }

    Ok(TransformerPipeline::new(transformers))
  }

  #[allow(unused)]
  fn validators(&self, _path: &Path) -> Result<Vec<Box<dyn ValidatorPlugin>>, anyhow::Error> {
    todo!()
  }
}

#[cfg(test)]
mod tests {
  use crate::test_utils::{config_plugins, make_test_plugin_context};

  use super::*;

  #[test]
  fn returns_bundler() {
    let bundler = config_plugins(make_test_plugin_context())
      .bundler()
      .expect("Not to panic");

    assert_eq!(format!("{:?}", bundler), "RpcBundlerPlugin")
  }

  #[test]
  fn returns_compressors() {
    let compressors = config_plugins(make_test_plugin_context())
      .compressors(Path::new("a.js"))
      .expect("Not to panic");

    assert_eq!(format!("{:?}", compressors), "[RpcCompressorPlugin]")
  }

  #[test]
  fn returns_namers() {
    let namers = config_plugins(make_test_plugin_context())
      .namers()
      .expect("Not to panic");

    assert_eq!(format!("{:?}", namers), "[RpcNamerPlugin]")
  }

  #[test]
  fn returns_optimizers() {
    let optimizers = config_plugins(make_test_plugin_context())
      .optimizers(Path::new("a.js"), None)
      .expect("Not to panic");

    assert_eq!(format!("{:?}", optimizers), "[RpcOptimizerPlugin]")
  }

  #[test]
  fn returns_packager() {
    let packager = config_plugins(make_test_plugin_context())
      .packager(Path::new("a.js"))
      .expect("Not to panic");

    assert_eq!(format!("{:?}", packager), "RpcPackagerPlugin")
  }

  #[test]
  fn returns_reporter() {
    let reporter = config_plugins(make_test_plugin_context()).reporter();

    assert_eq!(
      format!("{:?}", reporter),
      "CompositeReporterPlugin { reporters: [RpcReporterPlugin] }"
    )
  }

  #[test]
  fn returns_resolvers() {
    let resolvers = config_plugins(make_test_plugin_context())
      .resolvers()
      .expect("Not to panic");

    assert_eq!(format!("{:?}", resolvers), "[AtlaspackResolver]")
  }

  #[test]
  fn returns_runtimes() {
    let runtimes = config_plugins(make_test_plugin_context())
      .runtimes()
      .expect("Not to panic");

    assert_eq!(format!("{:?}", runtimes), "[RpcRuntimePlugin]")
  }

  #[test]
  fn returns_transformers() {
    let pipeline = config_plugins(make_test_plugin_context())
      .transformers(Path::new("a.ts"), None)
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
